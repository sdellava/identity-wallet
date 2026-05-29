use crate::state::actions::ActionTrait;
use crate::state::iota_wallet::reducers::create_identity::create_identity;
use crate::{reducer, state::Reducer};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, TS, Clone)]
#[ts(export, export_to = "bindings/actions/CreateIotaIdentity.ts")]
pub struct CreateIotaIdentity {}

#[typetag::serde(name = "[IOTA Wallet] Create identity")]
impl ActionTrait for CreateIotaIdentity {
    fn reducers<'a>(&self) -> Vec<Reducer<'a>> {
        vec![reducer!(create_identity)]
    }
}
