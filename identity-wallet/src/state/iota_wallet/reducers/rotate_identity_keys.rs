use crate::{
    error::AppError,
    state::{
        actions::{listen, Action},
        iota_wallet::{
            actions::rotate_identity_keys::RotateIotaIdentityKeys,
            reducers::{
                create_identity::{
                    identity_client_for_wallet, replace_identity_document_controller_key,
                    update_did_document_with_gas_station,
                },
                create_or_load_wallet::{load_stored_wallet, save_stored_wallet},
            },
            IotaWalletState,
        },
        AppState,
    },
};

use identity_iota::iota::IotaDID;
use std::str::FromStr;

pub async fn rotate_identity_keys(state: AppState, action: Action) -> Result<AppState, AppError> {
    if listen::<RotateIotaIdentityKeys>(action).is_none() {
        return Ok(state);
    }

    let mut wallet = load_stored_wallet(&state).await?;
    let Some(did) = wallet.did.clone() else {
        return Ok(AppState {
            iota_wallet: IotaWalletState {
                identity_rotation_status: Some("failed".to_string()),
                last_error: Some("No DID has been created yet.".to_string()),
                ..IotaWalletState::from(&wallet)
            },
            ..state
        });
    };

    let identity_client = identity_client_for_wallet(&wallet).await?;
    let did = IotaDID::from_str(&did).map_err(|e| AppError::Error(format!("Stored IOTA DID is invalid: {e}")))?;
    let mut document = identity_client
        .resolve_did(&did)
        .await
        .map_err(|e| AppError::Error(format!("Failed to resolve DID before key rotation: {e}")))?;

    replace_identity_document_controller_key(&mut wallet, &mut document)?;

    let document = update_did_document_with_gas_station(&wallet, &identity_client, document)
        .await
        .map_err(|e| AppError::Error(format!("Failed to publish rotated IOTA identity keys: {e}")))?;

    wallet.did_document = serde_json::to_string_pretty(&document).ok();
    save_stored_wallet(&state, &wallet).await?;

    Ok(AppState {
        iota_wallet: IotaWalletState {
            identity_rotation_status: Some("success".to_string()),
            identity_validation_status: Some("valid".to_string()),
            identity_validation_error: None,
            last_error: None,
            ..IotaWalletState::from(&wallet)
        },
        ..state
    })
}
