use crate::{
    error::AppError,
    state::{
        actions::{listen, Action},
        iota_wallet::{IotaWalletState, StoredIotaWallet},
        AppState,
    },
};

use iota_keys::keystore::{AccountKeystore, InMemKeystore};
use iota_sdk::types::crypto::{EncodeDecodeBase64, SignatureScheme};

pub const IOTA_WALLET_STORE_KEY: &str = "iota-wallet";

pub async fn create_or_load_wallet(state: AppState, action: Action) -> Result<AppState, AppError> {
    let Some(payload) =
        listen::<crate::state::iota_wallet::actions::create_or_load_wallet::CreateOrLoadIotaWallet>(action)
    else {
        return Ok(state);
    };

    let stronghold_manager = state
        .core_utils
        .managers
        .lock()
        .await
        .stronghold_manager
        .clone()
        .ok_or(AppError::MissingManagerError("stronghold"))?;

    let mut wallet = match stronghold_manager
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
            let public_key = keystore
                .get_key(&address)
                .map_err(|e| AppError::Error(format!("Failed to read generated IOTA public key: {e}")))?
                .public()
                .encode_base64();
            let wallet = StoredIotaWallet {
                mnemonic,
                address: address.to_string(),
                public_key: Some(public_key),
                network: payload.network,
                did: None,
                did_network: None,
                did_document: None,
                identity_controller_cap: None,
            };
            let bytes = serde_json::to_vec(&wallet).map_err(AppError::DeserializeFailed)?;
            stronghold_manager
                .insert_named(IOTA_WALLET_STORE_KEY, bytes)
                .map_err(AppError::StrongholdInsertionError)?;
            wallet
        }
    };

    if wallet.network != payload.network {
        wallet.network = payload.network;
        save_stored_wallet(&state, &wallet).await?;
    }

    Ok(AppState {
        iota_wallet: IotaWalletState::from(&wallet),
        ..state
    })
}

pub(crate) async fn load_stored_wallet(state: &AppState) -> Result<StoredIotaWallet, AppError> {
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

pub(crate) async fn save_stored_wallet(state: &AppState, wallet: &StoredIotaWallet) -> Result<(), AppError> {
    let managers = state.core_utils.managers.lock().await;
    let stronghold_manager = managers
        .stronghold_manager
        .clone()
        .ok_or(AppError::MissingManagerError("stronghold"))?;
    drop(managers);

    let bytes = serde_json::to_vec(wallet).map_err(AppError::DeserializeFailed)?;
    stronghold_manager
        .insert_named(IOTA_WALLET_STORE_KEY, bytes)
        .map_err(AppError::StrongholdInsertionError)
}
