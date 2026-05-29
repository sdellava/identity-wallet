use crate::state::actions::ActionTrait;
use crate::state::iota_wallet::reducers::submit_wallet_login::submit_wallet_login;
use crate::{reducer, state::Reducer};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, TS, Clone)]
#[ts(export, export_to = "bindings/actions/SubmitWalletLogin.ts")]
pub struct SubmitWalletLogin {}

#[typetag::serde(name = "[IOTA Wallet] Submit wallet login")]
impl ActionTrait for SubmitWalletLogin {
    fn reducers<'a>(&self) -> Vec<Reducer<'a>> {
        vec![reducer!(submit_wallet_login)]
    }
}
