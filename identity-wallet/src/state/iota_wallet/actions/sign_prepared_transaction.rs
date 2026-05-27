use crate::state::{actions::ActionTrait, iota_wallet::IotaNetwork};
use crate::state::iota_wallet::reducers::sign_prepared_transaction::sign_prepared_transaction;
use crate::{reducer, state::Reducer};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, TS, Clone)]
#[ts(export, export_to = "bindings/actions/SignPreparedIotaTransaction.ts")]
pub struct SignPreparedIotaTransaction {
    pub tx_data_bcs_base64: String,
    #[serde(default)]
    pub network: IotaNetwork,
    #[serde(default)]
    pub submit: bool,
}

#[typetag::serde(name = "[IOTA Wallet] Sign prepared transaction")]
impl ActionTrait for SignPreparedIotaTransaction {
    fn reducers<'a>(&self) -> Vec<Reducer<'a>> {
        vec![reducer!(sign_prepared_transaction)]
    }
}
