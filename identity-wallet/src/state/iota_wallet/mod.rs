pub mod actions;
pub mod reducers;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Default, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, TS)]
#[ts(export)]
#[serde(rename_all = "lowercase")]
pub enum IotaNetwork {
    #[default]
    Testnet,
    Mainnet,
}

impl IotaNetwork {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Testnet => "testnet",
            Self::Mainnet => "mainnet",
        }
    }
}

#[derive(Default, Serialize, Deserialize, Debug, Clone, TS)]
#[ts(export)]
#[serde(default)]
pub struct IotaWalletState {
    pub network: IotaNetwork,
    pub address: Option<String>,
    pub public_key: Option<String>,
    pub seed_phrase: Option<String>,
    pub did: Option<String>,
    pub did_document: Option<String>,
    pub identity_controller_cap: Option<String>,
    pub identity_validation_status: Option<String>,
    pub identity_validation_error: Option<String>,
    pub identity_rotation_status: Option<String>,
    pub identity_destruction_status: Option<String>,
    pub faucet_status: Option<String>,
    pub last_transaction_digest: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct StoredIotaWallet {
    pub mnemonic: String,
    pub address: String,
    pub public_key: Option<String>,
    #[serde(default)]
    pub identity_controller_private_key: Option<String>,
    #[serde(default)]
    pub identity_controller_public_jwk: Option<String>,
    #[serde(default)]
    pub network: IotaNetwork,
    pub did: Option<String>,
    pub did_network: Option<IotaNetwork>,
    #[serde(default)]
    pub did_document: Option<String>,
    pub identity_controller_cap: Option<String>,
}

impl From<&StoredIotaWallet> for IotaWalletState {
    fn from(wallet: &StoredIotaWallet) -> Self {
        let did = wallet
            .did_network
            .filter(|network| *network == wallet.network)
            .and_then(|_| wallet.did.clone());
        let identity_controller_cap = if did.is_some() {
            wallet.identity_controller_cap.clone()
        } else {
            None
        };
        let did_document = if did.is_some() {
            wallet.did_document.clone()
        } else {
            None
        };

        Self {
            network: wallet.network,
            address: Some(wallet.address.clone()),
            public_key: wallet.public_key.clone(),
            seed_phrase: Some(wallet.mnemonic.clone()),
            did,
            did_document,
            identity_controller_cap,
            identity_validation_status: None,
            identity_validation_error: None,
            identity_rotation_status: None,
            identity_destruction_status: None,
            faucet_status: None,
            last_transaction_digest: None,
            last_error: None,
        }
    }
}
