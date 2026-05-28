use crate::{
    error::AppError,
    state::{
        actions::{listen, Action},
        iota_wallet::{
            actions::request_faucet_funds::RequestIotaFaucetFunds, reducers::create_or_load_wallet::load_stored_wallet,
            IotaNetwork, IotaWalletState,
        },
        AppState,
    },
};

use serde_json::json;

const IOTA_TESTNET_FAUCET_URL: &str = "https://faucet.testnet.iota.cafe/v1/gas";

pub async fn request_faucet_funds(state: AppState, action: Action) -> Result<AppState, AppError> {
    if listen::<RequestIotaFaucetFunds>(action).is_none() {
        return Ok(state);
    }

    let wallet = load_stored_wallet(&state).await?;

    if wallet.network != IotaNetwork::Testnet {
        return Ok(AppState {
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
                faucet_status: None,
                last_transaction_digest: None,
                last_error: Some("The faucet is only available on IOTA testnet.".to_string()),
            },
            ..state
        });
    }

    let client = reqwest::Client::new();
    let response = client
        .post(IOTA_TESTNET_FAUCET_URL)
        .json(&json!({
            "FixedAmountRequest": {
                "recipient": wallet.address
            }
        }))
        .send()
        .await
        .map_err(|e| AppError::Error(format!("Failed to call IOTA testnet faucet: {e}")))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "Unable to read faucet response body".to_string());

    if !status.is_success() {
        return Ok(AppState {
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
                faucet_status: None,
                last_transaction_digest: None,
                last_error: Some(format!("Faucet request failed ({status}): {body}")),
            },
            ..state
        });
    }

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
            faucet_status: Some(body),
            last_transaction_digest: None,
            last_error: None,
        },
        ..state
    })
}
