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
    pub identity_controller_cap: Option<String>,
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
    pub network: IotaNetwork,
    pub did: Option<String>,
    pub did_network: Option<IotaNetwork>,
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

        Self {
            network: wallet.network,
            address: Some(wallet.address.clone()),
            public_key: wallet.public_key.clone(),
            seed_phrase: Some(wallet.mnemonic.clone()),
            did,
            identity_controller_cap,
            faucet_status: None,
            last_transaction_digest: None,
            last_error: None,
        }
    }
}
