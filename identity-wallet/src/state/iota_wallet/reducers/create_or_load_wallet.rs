use crate::{
    error::AppError,
    state::{
        actions::{listen, Action},
        iota_wallet::{IotaWalletState, StoredIotaWallet},
        AppState,
    },
};

use iota_keys::keystore::{AccountKeystore, InMemKeystore};
use iota_sdk::types::crypto::SignatureScheme;

pub const IOTA_WALLET_STORE_KEY: &str = "iota-wallet";

pub async fn create_or_load_wallet(state: AppState, action: Action) -> Result<AppState, AppError> {
    if listen::<crate::state::iota_wallet::actions::create_or_load_wallet::CreateOrLoadIotaWallet>(action).is_none() {
        return Ok(state);
    }

    let stronghold_manager = state
        .core_utils
        .managers
        .lock()
        .await
        .stronghold_manager
        .clone()
        .ok_or(AppError::MissingManagerError("stronghold"))?;

    let wallet = match stronghold_manager
        .get_named(IOTA_WALLET_STORE_KEY)
        .map_err(AppError::StrongholdValuesError)?
    {
        Some(bytes) => serde_json::from_slice::<StoredIotaWallet>(&bytes).map_err(AppError::DeserializeFailed)?,
        None => {
            let mut keystore = InMemKeystore::default();
            let (address, mnemonic, _) = keystore
                .generate_and_add_new_key(
                    SignatureScheme::ED25519,
                    Some("unime-iota".to_string()),
                    None,
                    Some("word24".to_string()),
                )
                .map_err(|e| AppError::Error(format!("Failed to create IOTA wallet: {e}")))?;
            let wallet = StoredIotaWallet {
                mnemonic,
                address: address.to_string(),
                did: Some(format!("did:iota:testnet:{}", address)),
                identity_controller_cap: None,
            };
            let bytes = serde_json::to_vec(&wallet).map_err(AppError::DeserializeFailed)?;
            stronghold_manager
                .insert_named(IOTA_WALLET_STORE_KEY, bytes)
                .map_err(AppError::StrongholdInsertionError)?;
            wallet
        }
    };

    Ok(AppState {
        iota_wallet: IotaWalletState::from(&wallet),
        ..state
    })
}

pub async fn load_stored_wallet(state: &AppState) -> Result<StoredIotaWallet, AppError> {
    let managers = state.core_utils.managers.lock().await;
    let stronghold_manager = managers
        .stronghold_manager
        .clone()
        .ok_or(AppError::MissingManagerError("stronghold"))?;
    drop(managers);

    let bytes = stronghold_manager
        .get_named(IOTA_WALLET_STORE_KEY)
        .map_err(AppError::StrongholdValuesError)?
        .ok_or_else(|| AppError::Error("IOTA wallet has not been initialized yet".to_string()))?;

    serde_json::from_slice::<StoredIotaWallet>(&bytes).map_err(AppError::DeserializeFailed)
}
