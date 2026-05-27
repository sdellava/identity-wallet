pub mod actions;
pub mod reducers;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Default, Serialize, Deserialize, Debug, Clone, TS)]
#[ts(export)]
#[serde(default)]
pub struct IotaWalletState {
    pub address: Option<String>,
    pub did: Option<String>,
    pub identity_controller_cap: Option<String>,
    pub last_transaction_digest: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct StoredIotaWallet {
    pub mnemonic: String,
    pub address: String,
    pub did: Option<String>,
    pub identity_controller_cap: Option<String>,
}

impl From<&StoredIotaWallet> for IotaWalletState {
    fn from(wallet: &StoredIotaWallet) -> Self {
        Self {
            address: Some(wallet.address.clone()),
            did: wallet.did.clone(),
            identity_controller_cap: wallet.identity_controller_cap.clone(),
            last_transaction_digest: None,
            last_error: None,
        }
    }
}
