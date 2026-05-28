use crate::state::actions::ActionTrait;
use crate::state::iota_wallet::reducers::rotate_identity_keys::rotate_identity_keys;
use crate::{reducer, state::Reducer};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, TS, Clone)]
#[ts(export, export_to = "bindings/actions/RotateIotaIdentityKeys.ts")]
pub struct RotateIotaIdentityKeys {}

#[typetag::serde(name = "[IOTA Wallet] Rotate identity keys")]
impl ActionTrait for RotateIotaIdentityKeys {
    fn reducers<'a>(&self) -> Vec<Reducer<'a>> {
        vec![reducer!(rotate_identity_keys)]
    }
}
