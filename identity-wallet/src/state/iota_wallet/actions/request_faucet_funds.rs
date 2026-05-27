use crate::state::actions::ActionTrait;
use crate::state::iota_wallet::reducers::request_faucet_funds::request_faucet_funds;
use crate::{reducer, state::Reducer};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, TS, Clone)]
#[ts(export, export_to = "bindings/actions/RequestIotaFaucetFunds.ts")]
pub struct RequestIotaFaucetFunds {}

#[typetag::serde(name = "[IOTA Wallet] Request faucet funds")]
impl ActionTrait for RequestIotaFaucetFunds {
    fn reducers<'a>(&self) -> Vec<Reducer<'a>> {
        vec![reducer!(request_faucet_funds)]
    }
}
