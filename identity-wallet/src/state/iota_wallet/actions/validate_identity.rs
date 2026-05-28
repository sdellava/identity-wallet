use crate::state::actions::ActionTrait;
use crate::state::iota_wallet::reducers::validate_identity::validate_identity;
use crate::{reducer, state::Reducer};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, TS, Clone)]
#[ts(export, export_to = "bindings/actions/ValidateIotaIdentity.ts")]
pub struct ValidateIotaIdentity {}

#[typetag::serde(name = "[IOTA Wallet] Validate identity")]
impl ActionTrait for ValidateIotaIdentity {
    fn reducers<'a>(&self) -> Vec<Reducer<'a>> {
        vec![reducer!(validate_identity)]
    }
}
