use crate::state::actions::ActionTrait;
use crate::state::iota_wallet::reducers::destroy_identity::destroy_identity;
use crate::{reducer, state::Reducer};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, TS, Clone)]
#[ts(export, export_to = "bindings/actions/DestroyIotaIdentity.ts")]
pub struct DestroyIotaIdentity {}

#[typetag::serde(name = "[IOTA Wallet] Destroy identity")]
impl ActionTrait for DestroyIotaIdentity {
    fn reducers<'a>(&self) -> Vec<Reducer<'a>> {
        vec![reducer!(destroy_identity)]
    }
}
