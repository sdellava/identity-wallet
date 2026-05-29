use crate::{
    error::AppError,
    state::{
        actions::{listen, Action},
        iota_wallet::{
            actions::sign_prepared_transaction::SignPreparedIotaTransaction,
            reducers::{
                create_identity::{gas_stations_for_network, IotaGasStation},
                create_or_load_wallet::load_stored_wallet,
            },
            IotaWalletState,
        },
        AppState,
    },
};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use iota_keys::keystore::{AccountKeystore, InMemKeystore};
use iota_sdk::{
    rpc_types::{IotaObjectRef, IotaTransactionBlockEffects, IotaTransactionBlockEffectsAPI},
    types::{
        base_types::IotaAddress,
        crypto::{EncodeDecodeBase64, SignatureScheme},
        transaction::{TransactionData, TransactionDataAPI, TransactionKind},
    },
};
use iota_sdk_types::crypto::Intent;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

const GAS_RESERVATION_DURATION_SECS: u64 = 60;

#[derive(Debug, Deserialize)]
struct ReserveGasResponse {
    result: Option<ReserveGasResult>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReserveGasResult {
    #[serde(alias = "sponsorAddress")]
    sponsor_address: IotaAddress,
    #[serde(alias = "reservationId")]
    reservation_id: u64,
    #[serde(alias = "gasCoins")]
    gas_coins: Vec<IotaObjectRef>,
}

#[derive(Debug, Serialize)]
struct ReserveGasRequest {
    gas_budget: u64,
    reserve_duration_secs: u64,
}

#[derive(Debug, Serialize)]
struct ExecuteSponsoredTxRequest {
    reservation_id: u64,
    tx_bytes: String,
    user_sig: String,
    request_type: &'static str,
}

#[derive(Debug, Deserialize)]
struct ExecuteSponsoredTxResponse {
    effects: Option<IotaTransactionBlockEffects>,
    error: Option<String>,
}

pub async fn sign_prepared_transaction(state: AppState, action: Action) -> Result<AppState, AppError> {
    let Some(payload) = listen::<SignPreparedIotaTransaction>(action) else {
        return Ok(state);
    };

    let wallet = load_stored_wallet(&state).await?;
    if wallet.network != payload.network {
        return Ok(AppState {
            iota_wallet: IotaWalletState {
                network: wallet.network,
                address: Some(wallet.address),
                public_key: wallet.public_key,
                seed_phrase: Some(wallet.mnemonic),
                did: wallet.did,
                did_document: wallet.did_document,
                identity_controller_cap: wallet.identity_controller_cap,
                faucet_status: state.iota_wallet.faucet_status,
                last_error: Some(format!(
                    "Prepared transaction is for {}, but the wallet is set to {}.",
                    payload.network.as_str(),
                    wallet.network.as_str()
                )),
                ..state.iota_wallet
            },
            ..state
        });
    }

    let mut keystore = InMemKeystore::default();
    let address = keystore
        .import_from_mnemonic(
            &wallet.mnemonic,
            SignatureScheme::ED25519,
            None,
            Some("unime-iota".to_string()),
        )
        .map_err(|e| AppError::Error(format!("Failed to restore IOTA key from mnemonic: {e}")))?;

    let expected_sender = IotaAddress::from_str(&wallet.address)
        .map_err(|e| AppError::Error(format!("Stored IOTA address is invalid: {e}")))?;
    let tx_data = if let Some(tx_data_bcs_base64) = &payload.tx_data_bcs_base64 {
        let tx_bytes = STANDARD
            .decode(tx_data_bcs_base64.as_bytes())
            .map_err(|e| AppError::Error(format!("Invalid transaction BCS base64: {e}")))?;
        bcs::from_bytes(&tx_bytes).map_err(|e| AppError::Error(format!("Invalid IOTA TransactionData BCS: {e}")))?
    } else if let Some(tx_kind_bcs_base64) = &payload.tx_kind_bcs_base64 {
        let tx_bytes = STANDARD
            .decode(tx_kind_bcs_base64.as_bytes())
            .map_err(|e| AppError::Error(format!("Invalid transaction kind BCS base64: {e}")))?;
        let tx_kind: TransactionKind = bcs::from_bytes(&tx_bytes)
            .map_err(|e| AppError::Error(format!("Invalid IOTA TransactionKind BCS: {e}")))?;
        let gas_price = payload
            .gas_price
            .ok_or_else(|| AppError::Error("Prepared transaction kind is missing gas_price.".to_string()))?;
        TransactionData::new_with_gas_coins_allow_sponsor(
            tx_kind,
            expected_sender,
            Vec::new(),
            payload.gas_budget,
            gas_price,
            expected_sender,
        )
    } else {
        return Ok(AppState {
            iota_wallet: IotaWalletState {
                network: wallet.network,
                address: Some(wallet.address),
                public_key: wallet.public_key,
                seed_phrase: Some(wallet.mnemonic),
                did: wallet.did,
                did_document: wallet.did_document,
                identity_controller_cap: wallet.identity_controller_cap,
                faucet_status: state.iota_wallet.faucet_status,
                last_error: Some("Prepared transaction payload does not contain transaction data.".to_string()),
                ..state.iota_wallet
            },
            ..state
        });
    };

    if address != expected_sender || tx_data.sender() != expected_sender {
        return Ok(AppState {
            iota_wallet: IotaWalletState {
                network: wallet.network,
                address: Some(wallet.address),
                public_key: wallet.public_key,
                seed_phrase: Some(wallet.mnemonic),
                did: wallet.did,
                did_document: wallet.did_document,
                identity_controller_cap: wallet.identity_controller_cap,
                faucet_status: state.iota_wallet.faucet_status,
                last_error: Some("Prepared transaction sender does not match this wallet address".to_string()),
                ..state.iota_wallet
            },
            ..state
        });
    }

    let digest = if payload.submit {
        Some(execute_prepared_transaction_with_gas_station(&keystore, expected_sender, tx_data, wallet.network).await?)
    } else {
        keystore
            .sign_secure(&expected_sender, &tx_data, Intent::iota_transaction())
            .map_err(|e| AppError::Error(format!("Failed to sign IOTA transaction: {e}")))?;
        None
    };

    Ok(AppState {
        iota_wallet: IotaWalletState {
            network: wallet.network,
            address: Some(wallet.address),
            public_key: wallet.public_key,
            seed_phrase: Some(wallet.mnemonic),
            did: wallet.did,
            did_document: wallet.did_document,
            identity_controller_cap: wallet.identity_controller_cap,
            identity_validation_status: state.iota_wallet.identity_validation_status,
            identity_validation_error: state.iota_wallet.identity_validation_error,
            identity_rotation_status: state.iota_wallet.identity_rotation_status,
            identity_destruction_status: state.iota_wallet.identity_destruction_status,
            faucet_status: state.iota_wallet.faucet_status,
            last_transaction_digest: digest,
            last_error: None,
        },
        ..state
    })
}

async fn execute_prepared_transaction_with_gas_station(
    keystore: &InMemKeystore,
    sender: IotaAddress,
    tx_data: TransactionData,
    network: crate::state::iota_wallet::IotaNetwork,
) -> Result<String, AppError> {
    let http_client = reqwest::Client::new();
    let mut last_error = None;

    for gas_station in gas_stations_for_network(network) {
        match execute_prepared_transaction_with_one_gas_station(
            &http_client,
            keystore,
            sender,
            tx_data.clone(),
            gas_station,
        )
        .await
        {
            Ok(digest) => return Ok(digest),
            Err(error) => {
                last_error = Some(format!("{} failed: {error}", gas_station.url));
            }
        }
    }

    Err(AppError::Error(format!(
        "All configured IOTA gas stations failed. {}",
        last_error.unwrap_or_else(|| "No gas station was attempted.".to_string())
    )))
}

async fn execute_prepared_transaction_with_one_gas_station(
    http_client: &reqwest::Client,
    keystore: &InMemKeystore,
    sender: IotaAddress,
    mut tx_data: TransactionData,
    gas_station: &IotaGasStation,
) -> Result<String, AppError> {
    let gas_budget = tx_data.gas_budget();
    let reservation = reserve_gas(http_client, gas_station, gas_budget).await?;

    {
        let gas_data = tx_data.gas_data_mut();
        gas_data.owner = reservation.sponsor_address;
        gas_data.payment = reservation
            .gas_coins
            .into_iter()
            .map(|coin| coin.to_object_ref())
            .collect();
        gas_data.budget = gas_budget;
    }

    let signature = keystore
        .sign_secure(&sender, &tx_data, Intent::iota_transaction())
        .map_err(|e| AppError::Error(format!("Failed to sign sponsored IOTA transaction: {e}")))?;

    let tx_bytes = bcs::to_bytes(&tx_data)
        .map_err(|e| AppError::Error(format!("Failed to encode sponsored IOTA transaction: {e}")))?;
    let response = http_client
        .post(format!("{}/v1/execute_tx", gas_station.url.trim_end_matches('/')))
        .bearer_auth(gas_station.token)
        .json(&ExecuteSponsoredTxRequest {
            reservation_id: reservation.reservation_id,
            tx_bytes: STANDARD.encode(tx_bytes),
            user_sig: signature.encode_base64(),
            request_type: "WaitForLocalExecution",
        })
        .send()
        .await
        .map_err(|e| AppError::Error(format!("Failed to call IOTA gas station execute_tx: {e}")))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "Unable to read gas station response body".to_string());
    if !status.is_success() {
        return Err(AppError::Error(format!(
            "Gas station execute_tx failed ({status}): {body}"
        )));
    }

    let response: ExecuteSponsoredTxResponse = serde_json::from_str(&body)
        .map_err(|e| AppError::Error(format!("Invalid gas station execute_tx response: {e}: {body}")))?;
    let effects = response.effects.ok_or_else(|| {
        AppError::Error(
            response
                .error
                .unwrap_or_else(|| "Gas station did not return effects".to_string()),
        )
    })?;

    Ok(effects.transaction_digest().to_string())
}

async fn reserve_gas(
    http_client: &reqwest::Client,
    gas_station: &IotaGasStation,
    gas_budget: u64,
) -> Result<ReserveGasResult, AppError> {
    let response = http_client
        .post(format!("{}/v1/reserve_gas", gas_station.url.trim_end_matches('/')))
        .bearer_auth(gas_station.token)
        .json(&ReserveGasRequest {
            gas_budget,
            reserve_duration_secs: GAS_RESERVATION_DURATION_SECS,
        })
        .send()
        .await
        .map_err(|e| AppError::Error(format!("Failed to reserve IOTA gas: {e}")))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "Unable to read gas station response body".to_string());
    if !status.is_success() {
        return Err(AppError::Error(format!(
            "Gas station reserve_gas failed ({status}): {body}"
        )));
    }

    let response: ReserveGasResponse = serde_json::from_str(&body)
        .map_err(|e| AppError::Error(format!("Invalid gas station reserve_gas response: {e}: {body}")))?;
    response.result.ok_or_else(|| {
        AppError::Error(
            response
                .error
                .unwrap_or_else(|| "Gas station did not reserve gas".to_string()),
        )
    })
}
