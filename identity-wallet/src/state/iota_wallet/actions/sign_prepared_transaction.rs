use crate::state::iota_wallet::reducers::sign_prepared_transaction::sign_prepared_transaction;
use crate::state::{actions::ActionTrait, iota_wallet::IotaNetwork};
use crate::{reducer, state::Reducer};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, TS, Clone)]
#[ts(export, export_to = "bindings/actions/SignPreparedIotaTransaction.ts")]
pub struct SignPreparedIotaTransaction {
    #[serde(default)]
    pub tx_data_bcs_base64: Option<String>,
    #[serde(default)]
    pub tx_kind_bcs_base64: Option<String>,
    #[serde(default = "default_gas_budget")]
    pub gas_budget: u64,
    #[serde(default)]
    pub gas_price: Option<u64>,
    #[serde(default)]
    pub network: IotaNetwork,
    #[serde(default)]
    pub submit: bool,
}

fn default_gas_budget() -> u64 {
    5_000_000
}

#[typetag::serde(name = "[IOTA Wallet] Sign prepared transaction")]
impl ActionTrait for SignPreparedIotaTransaction {
    fn reducers<'a>(&self) -> Vec<Reducer<'a>> {
        vec![reducer!(sign_prepared_transaction)]
    }
}
