use crate::{
    error::AppError,
    state::{
        actions::{listen, Action},
        iota_wallet::{
            actions::destroy_identity::DestroyIotaIdentity,
            reducers::{
                create_identity::{execute_with_wallet_gas_station, identity_client_for_wallet},
                create_or_load_wallet::{load_stored_wallet, save_stored_wallet},
            },
            IotaWalletState,
        },
        AppState,
    },
};

use async_trait::async_trait;
use identity_iota::{
    iota::{rebased::Error as RebasedIdentityError, IotaDID},
    iota_interaction::{
        ident_str,
        rpc_types::IotaTransactionBlockEffects,
        types::{
            base_types::{ObjectID, STD_OPTION_MODULE_NAME},
            object::Owner,
            programmable_transaction_builder::ProgrammableTransactionBuilder as Ptb,
            transaction::{Argument, ObjectArg, ProgrammableTransaction},
            TypeTag, IOTA_FRAMEWORK_PACKAGE_ID, MOVE_STDLIB_PACKAGE_ID,
        },
        OptionalSync,
    },
};
use product_common::{
    core_client::CoreClientReadOnly,
    transaction::transaction_builder::{Transaction, TransactionBuilder},
};
use std::str::FromStr;

#[derive(Debug)]
struct DestroyIdentityTx {
    identity_id: ObjectID,
    controller_cap_id: ObjectID,
}

#[async_trait]
impl Transaction for DestroyIdentityTx {
    type Error = RebasedIdentityError;
    type Output = ();

    async fn build_programmable_transaction<C>(&self, client: &C) -> Result<ProgrammableTransaction, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let identity_ref = client
            .get_object_ref_by_id(self.identity_id)
            .await
            .map_err(|e| RebasedIdentityError::ObjectLookup(e.to_string()))?
            .ok_or_else(|| {
                RebasedIdentityError::ObjectLookup(format!("identity {} was not found", self.identity_id))
            })?;
        let controller_cap_ref = client
            .get_object_ref_by_id(self.controller_cap_id)
            .await
            .map_err(|e| RebasedIdentityError::ObjectLookup(e.to_string()))?
            .ok_or_else(|| {
                RebasedIdentityError::ObjectLookup(format!("controller cap {} was not found", self.controller_cap_id))
            })?;

        let Owner::Shared { initial_shared_version } = identity_ref.owner else {
            return Err(RebasedIdentityError::TransactionBuildingFailed(format!(
                "identity {} is not a shared object",
                self.identity_id
            )));
        };

        let mut ptb = Ptb::new();
        let identity = ptb
            .obj(ObjectArg::SharedObject {
                id: identity_ref.object_id(),
                initial_shared_version,
                mutable: true,
            })
            .map_err(|e| RebasedIdentityError::InvalidArgument(e.to_string()))?;
        let controller_cap = ptb
            .obj(ObjectArg::ImmOrOwnedObject(
                controller_cap_ref.reference.to_object_ref(),
            ))
            .map_err(|e| RebasedIdentityError::InvalidArgument(e.to_string()))?;
        let expiration = ptb.programmable_move_call(
            MOVE_STDLIB_PACKAGE_ID,
            STD_OPTION_MODULE_NAME.into(),
            ident_str!("none").into(),
            vec![TypeTag::U64],
            vec![],
        );
        let threshold = ptb.programmable_move_call(
            MOVE_STDLIB_PACKAGE_ID,
            STD_OPTION_MODULE_NAME.into(),
            ident_str!("none").into(),
            vec![TypeTag::U64],
            vec![],
        );
        let controllers_to_add = ptb.programmable_move_call(
            IOTA_FRAMEWORK_PACKAGE_ID,
            ident_str!("vec_map").into(),
            ident_str!("empty").into(),
            vec![TypeTag::Address, TypeTag::U64],
            vec![],
        );
        let controllers_to_remove = ptb
            .pure(vec![self.controller_cap_id])
            .map_err(|e| RebasedIdentityError::InvalidArgument(e.to_string()))?;
        let controllers_to_update = ptb.programmable_move_call(
            IOTA_FRAMEWORK_PACKAGE_ID,
            ident_str!("vec_map").into(),
            ident_str!("empty").into(),
            vec![
                TypeTag::from_str("0x2::object::ID").expect("valid object id type tag"),
                TypeTag::U64,
            ],
            vec![],
        );
        let Argument::Result(borrow_result) = ptb.programmable_move_call(
            client.package_id(),
            ident_str!("controller").into(),
            ident_str!("borrow").into(),
            vec![],
            vec![controller_cap],
        ) else {
            unreachable!("move calls always return a result argument");
        };
        let delegation_token = Argument::NestedResult(borrow_result, 0);
        let borrow = Argument::NestedResult(borrow_result, 1);

        ptb.programmable_move_call(
            client.package_id(),
            ident_str!("identity").into(),
            ident_str!("propose_config_change").into(),
            vec![],
            vec![
                identity,
                delegation_token,
                expiration,
                threshold,
                controllers_to_add,
                controllers_to_remove,
                controllers_to_update,
            ],
        );
        ptb.programmable_move_call(
            client.package_id(),
            ident_str!("controller").into(),
            ident_str!("put_back").into(),
            vec![],
            vec![controller_cap, delegation_token, borrow],
        );

        ptb.programmable_move_call(
            client.package_id(),
            ident_str!("identity").into(),
            ident_str!("destroy_controller_cap").into(),
            vec![],
            vec![identity, controller_cap],
        );

        Ok(ptb.finish())
    }

    async fn apply<C>(
        self,
        _effects: &mut IotaTransactionBlockEffects,
        _client: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Ok(())
    }
}

pub async fn destroy_identity(state: AppState, action: Action) -> Result<AppState, AppError> {
    if listen::<DestroyIotaIdentity>(action).is_none() {
        return Ok(state);
    }

    let mut wallet = load_stored_wallet(&state).await?;
    let Some(did) = wallet.did.clone() else {
        return Ok(AppState {
            iota_wallet: IotaWalletState {
                identity_destruction_status: Some("failed".to_string()),
                last_error: Some("No DID has been created yet.".to_string()),
                ..IotaWalletState::from(&wallet)
            },
            ..state
        });
    };
    let Some(controller_cap_id) = wallet.identity_controller_cap.clone() else {
        return Ok(AppState {
            iota_wallet: IotaWalletState {
                identity_destruction_status: Some("failed".to_string()),
                last_error: Some("No identity controller cap is stored for this wallet.".to_string()),
                ..IotaWalletState::from(&wallet)
            },
            ..state
        });
    };

    let identity_client = identity_client_for_wallet(&wallet).await?;
    let did = IotaDID::from_str(&did).map_err(|e| AppError::Error(format!("Stored IOTA DID is invalid: {e}")))?;
    let identity_id = did.to_object_id();
    let controller_cap_id = ObjectID::from_str(&controller_cap_id)
        .map_err(|e| AppError::Error(format!("Stored IOTA controller cap is invalid: {e}")))?;

    execute_with_wallet_gas_station(&wallet, &identity_client, || async {
        Ok(TransactionBuilder::new(DestroyIdentityTx {
            identity_id,
            controller_cap_id,
        }))
    })
    .await
    .map_err(|e| AppError::Error(format!("Failed to destroy identity controller cap: {e}")))?;

    wallet.did = None;
    wallet.did_network = None;
    wallet.did_document = None;
    wallet.identity_controller_cap = None;
    wallet.identity_controller_private_key = None;
    wallet.identity_controller_public_jwk = None;
    save_stored_wallet(&state, &wallet).await?;

    Ok(AppState {
        iota_wallet: IotaWalletState {
            identity_destruction_status: Some("success".to_string()),
            identity_validation_status: None,
            identity_validation_error: None,
            identity_rotation_status: None,
            last_error: None,
            ..IotaWalletState::from(&wallet)
        },
        ..state
    })
}
