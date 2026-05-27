use crate::state::actions::ActionTrait;
use crate::state::iota_wallet::reducers::publish_did::publish_did;
use crate::{reducer, state::Reducer};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, TS, Clone)]
#[ts(export, export_to = "bindings/actions/PublishIotaDid.ts")]
pub struct PublishIotaDid {}

#[typetag::serde(name = "[IOTA Wallet] Publish DID")]
impl ActionTrait for PublishIotaDid {
    fn reducers<'a>(&self) -> Vec<Reducer<'a>> {
        vec![reducer!(publish_did)]
    }
}
