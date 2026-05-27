use crate::{
    error::AppError,
    state::{
        actions::{listen, Action},
        iota_wallet::{
            actions::sign_prepared_transaction::SignPreparedIotaTransaction,
            reducers::create_or_load_wallet::load_stored_wallet, IotaNetwork, IotaWalletState,
        },
        AppState,
    },
};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use iota_keys::keystore::{AccountKeystore, InMemKeystore};
use iota_sdk::{
    rpc_types::IotaTransactionBlockResponseOptions,
    types::{
        base_types::IotaAddress,
        crypto::SignatureScheme,
        quorum_driver_types::ExecuteTransactionRequestType,
        transaction::{Transaction, TransactionData, TransactionDataAPI},
    },
    IotaClientBuilder,
};
use iota_sdk_types::crypto::Intent;
use std::str::FromStr;

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

    let tx_bytes = STANDARD
        .decode(payload.tx_data_bcs_base64.as_bytes())
        .map_err(|e| AppError::Error(format!("Invalid transaction BCS base64: {e}")))?;
    let tx_data: TransactionData =
        bcs::from_bytes(&tx_bytes).map_err(|e| AppError::Error(format!("Invalid IOTA TransactionData BCS: {e}")))?;

    let expected_sender = IotaAddress::from_str(&wallet.address)
        .map_err(|e| AppError::Error(format!("Stored IOTA address is invalid: {e}")))?;
    if address != expected_sender || tx_data.sender() != expected_sender {
        return Ok(AppState {
            iota_wallet: IotaWalletState {
                network: wallet.network,
                address: Some(wallet.address),
                public_key: wallet.public_key,
                seed_phrase: Some(wallet.mnemonic),
                did: wallet.did,
                identity_controller_cap: wallet.identity_controller_cap,
                faucet_status: state.iota_wallet.faucet_status,
                last_error: Some("Prepared transaction sender does not match this wallet address".to_string()),
                ..state.iota_wallet
            },
            ..state
        });
    }

    let signature = keystore
        .sign_secure(&expected_sender, &tx_data, Intent::iota_transaction())
        .map_err(|e| AppError::Error(format!("Failed to sign IOTA transaction: {e}")))?;

    let digest = if payload.submit {
        let client = match payload.network {
            IotaNetwork::Testnet => IotaClientBuilder::default()
                .build_testnet()
                .await
                .map_err(|e| AppError::Error(format!("Failed to connect to IOTA testnet: {e}")))?,
            IotaNetwork::Mainnet => IotaClientBuilder::default()
                .build_mainnet()
                .await
                .map_err(|e| AppError::Error(format!("Failed to connect to IOTA mainnet: {e}")))?,
        };
        let response = client
            .quorum_driver_api()
            .execute_transaction_block(
                Transaction::from_data(tx_data, vec![signature]),
                IotaTransactionBlockResponseOptions::full_content(),
                ExecuteTransactionRequestType::WaitForLocalExecution,
            )
            .await
            .map_err(|e| AppError::Error(format!("Failed to submit IOTA transaction: {e}")))?;
        Some(response.digest.to_string())
    } else {
        None
    };

    Ok(AppState {
        iota_wallet: IotaWalletState {
            network: wallet.network,
            address: Some(wallet.address),
            public_key: wallet.public_key,
            seed_phrase: Some(wallet.mnemonic),
            did: wallet.did,
            identity_controller_cap: wallet.identity_controller_cap,
            faucet_status: state.iota_wallet.faucet_status,
            last_transaction_digest: digest,
            last_error: None,
        },
        ..state
    })
}
