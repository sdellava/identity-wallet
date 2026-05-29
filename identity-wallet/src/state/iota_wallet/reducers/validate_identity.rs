use crate::{
    error::AppError,
    state::{
        actions::{listen, Action},
        iota_wallet::{
            actions::validate_identity::ValidateIotaIdentity,
            reducers::{
                create_identity::identity_client_for_wallet,
                create_or_load_wallet::{load_stored_wallet, save_stored_wallet},
            },
            IotaWalletState,
        },
        AppState,
    },
};

use identity_iota::iota::rebased::migration::Identity;
use identity_iota::verification::MethodScope;
use std::str::FromStr;

pub async fn validate_identity(state: AppState, action: Action) -> Result<AppState, AppError> {
    if listen::<ValidateIotaIdentity>(action).is_none() {
        return Ok(state);
    }

    let mut wallet = load_stored_wallet(&state).await?;
    let Some(did) = wallet.did.clone() else {
        return Ok(AppState {
            iota_wallet: IotaWalletState {
                identity_validation_status: Some("invalid".to_string()),
                identity_validation_error: Some("No DID has been created yet.".to_string()),
                ..IotaWalletState::from(&wallet)
            },
            ..state
        });
    };

    let identity_client = identity_client_for_wallet(&wallet).await?;
    let did = identity_iota::iota::IotaDID::from_str(&did)
        .map_err(|e| AppError::Error(format!("Stored IOTA DID is invalid: {e}")))?;

    let document = match identity_client.resolve_did(&did).await {
        Ok(document) => document,
        Err(e) => {
            return Ok(AppState {
                iota_wallet: IotaWalletState {
                    identity_validation_status: Some("invalid".to_string()),
                    identity_validation_error: Some(format!("Unable to resolve DID document: {e}")),
                    ..IotaWalletState::from(&wallet)
                },
                ..state
            });
        }
    };

    let identity_controller_cap = match identity_client.get_identity(document.id().to_object_id()).await {
        Ok(Identity::FullFledged(identity)) => identity
            .get_controller_token(&identity_client)
            .await
            .ok()
            .flatten()
            .map(|token| token.id().to_string()),
        _ => None,
    };

    wallet.did_document = serde_json::to_string_pretty(&document).ok();
    wallet.identity_controller_cap = identity_controller_cap;
    let validation_error = if wallet.identity_controller_cap.is_none() {
        Some("Controller cap not found for this identity.".to_string())
    } else if document.controller().next().is_none() {
        Some("DID document has no controller.".to_string())
    } else if document.methods(Some(MethodScope::VerificationMethod)).is_empty() {
        Some("DID document has no verification method/public key.".to_string())
    } else {
        None
    };
    save_stored_wallet(&state, &wallet).await?;

    Ok(AppState {
        iota_wallet: match validation_error {
            Some(error) => IotaWalletState {
                identity_validation_status: Some("invalid".to_string()),
                identity_validation_error: Some(error),
                ..IotaWalletState::from(&wallet)
            },
            None => IotaWalletState {
                identity_validation_status: Some("valid".to_string()),
                identity_validation_error: None,
                ..IotaWalletState::from(&wallet)
            },
        },
        ..state
    })
}
