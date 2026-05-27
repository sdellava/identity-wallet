use crate::state::actions::ActionTrait;
use crate::state::iota_wallet::{reducers::create_or_load_wallet::create_or_load_wallet, IotaNetwork};
use crate::{reducer, state::Reducer};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, TS, Clone)]
#[ts(export, export_to = "bindings/actions/CreateOrLoadIotaWallet.ts")]
pub struct CreateOrLoadIotaWallet {
    #[serde(default)]
    pub network: IotaNetwork,
}

#[typetag::serde(name = "[IOTA Wallet] Create or load")]
impl ActionTrait for CreateOrLoadIotaWallet {
    fn reducers<'a>(&self) -> Vec<Reducer<'a>> {
        vec![reducer!(create_or_load_wallet)]
    }
}
