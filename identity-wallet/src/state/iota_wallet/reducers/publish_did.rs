use crate::{
    error::AppError,
    state::{
        actions::{listen, Action},
        iota_wallet::{
            actions::publish_did::PublishIotaDid,
            reducers::create_or_load_wallet::{load_stored_wallet, save_stored_wallet},
            IotaNetwork, IotaWalletState,
        },
        AppState,
    },
};

use identity_iota::{
    iota::{
        rebased::{client::IdentityClient, migration::Identity},
        IotaDocument,
    },
    iota_interaction::{
        types::{
            base_types::IotaAddress,
            crypto::{PublicKey, Signature},
            transaction::TransactionData,
        },
        IotaKeySignature,
    },
};
use iota_keys::keystore::{AccountKeystore, InMemKeystore};
use iota_sdk::{types::crypto::SignatureScheme, IotaClientBuilder};
use iota_sdk_types::crypto::Intent;
use secret_storage::{Error as SecretStorageError, Signer};

const IOTA_IDENTITY_GAS_BUDGET: u64 = 50_000_000;

#[derive(Clone, Copy)]
pub(crate) struct IotaGasStation {
    pub url: &'static str,
    pub token: &'static str,
}

struct IotaGasStationConfig {
    gas_station_1_url: &'static str,
    gas_station_1_token: &'static str,
    gas_station_2_url: &'static str,
    gas_station_2_token: &'static str,
}

const GAS_STATION_TESTNET: IotaGasStationConfig = IotaGasStationConfig {
    gas_station_1_url: "https://gas1.objectid.io",
    gas_station_1_token: "1111",
    gas_station_2_url: "https://gas2.objectid.io",
    gas_station_2_token: "1111",
};

const GAS_STATION_MAINNET: IotaGasStationConfig = IotaGasStationConfig {
    gas_station_1_url: "https://m-gas1.objectid.io",
    gas_station_1_token: "1111",
    gas_station_2_url: "https://m-gas2.objectid.io",
    gas_station_2_token: "1111",
};

pub(crate) fn gas_stations_for_network(network: IotaNetwork) -> &'static [IotaGasStation; 2] {
    const TESTNET_GAS_STATIONS: [IotaGasStation; 2] = [
        IotaGasStation {
            url: GAS_STATION_TESTNET.gas_station_1_url,
            token: GAS_STATION_TESTNET.gas_station_1_token,
        },
        IotaGasStation {
            url: GAS_STATION_TESTNET.gas_station_2_url,
            token: GAS_STATION_TESTNET.gas_station_2_token,
        },
    ];
    const MAINNET_GAS_STATIONS: [IotaGasStation; 2] = [
        IotaGasStation {
            url: GAS_STATION_MAINNET.gas_station_1_url,
            token: GAS_STATION_MAINNET.gas_station_1_token,
        },
        IotaGasStation {
            url: GAS_STATION_MAINNET.gas_station_2_url,
            token: GAS_STATION_MAINNET.gas_station_2_token,
        },
    ];

    match network {
        IotaNetwork::Testnet => &TESTNET_GAS_STATIONS,
        IotaNetwork::Mainnet => &MAINNET_GAS_STATIONS,
    }
}

struct WalletSigner {
    keystore: InMemKeystore,
    address: IotaAddress,
}

#[async_trait::async_trait]
impl Signer<IotaKeySignature> for WalletSigner {
    type KeyId = IotaAddress;

    async fn sign(&self, data: &TransactionData) -> Result<Signature, SecretStorageError> {
        self.keystore
            .sign_secure(&self.address, data, Intent::iota_transaction())
            .map_err(|e| SecretStorageError::Other(anyhow::anyhow!("failed to sign IOTA transaction: {e}")))
    }

    async fn public_key(&self) -> Result<PublicKey, SecretStorageError> {
        self.keystore
            .get_key(&self.address)
            .map(|key| key.public())
            .map_err(|e| SecretStorageError::Other(anyhow::anyhow!("failed to read public key: {e}")))
    }

    fn key_id(&self) -> Self::KeyId {
        self.address
    }
}

pub async fn publish_did(state: AppState, action: Action) -> Result<AppState, AppError> {
    if listen::<PublishIotaDid>(action).is_none() {
        return Ok(state);
    }

    let mut wallet = load_stored_wallet(&state).await?;
    let mut keystore = InMemKeystore::default();
    let address = keystore
        .import_from_mnemonic(
            &wallet.mnemonic,
            SignatureScheme::ED25519,
            None,
            Some("unime-iota".to_string()),
        )
        .map_err(|e| AppError::Error(format!("Failed to restore IOTA key from mnemonic: {e}")))?;

    if address.to_string() != wallet.address {
        return Ok(AppState {
            iota_wallet: IotaWalletState {
                network: wallet.network,
                address: Some(wallet.address),
                public_key: wallet.public_key,
                seed_phrase: Some(wallet.mnemonic),
                did: wallet.did,
                identity_controller_cap: wallet.identity_controller_cap,
                faucet_status: state.iota_wallet.faucet_status,
                last_error: Some("Stored IOTA mnemonic does not match the stored address.".to_string()),
                ..state.iota_wallet
            },
            ..state
        });
    }

    let signer = WalletSigner { keystore, address };

    let iota_client = match wallet.network {
        IotaNetwork::Testnet => IotaClientBuilder::default()
            .build_testnet()
            .await
            .map_err(|e| AppError::Error(format!("Failed to connect to IOTA testnet: {e}")))?,
        IotaNetwork::Mainnet => IotaClientBuilder::default()
            .build_mainnet()
            .await
            .map_err(|e| AppError::Error(format!("Failed to connect to IOTA mainnet: {e}")))?,
    };

    let identity_client = IdentityClient::from_iota_client(iota_client, None)
        .await
        .map_err(|e| AppError::Error(format!("Failed to create IOTA Identity client: {e}")))?
        .with_signer(signer)
        .await
        .map_err(|e| AppError::Error(format!("Failed to attach IOTA Identity signer: {e}")))?;

    let document = IotaDocument::new(identity_client.network());
    let document = identity_client
        .publish_did_document(document)
        .with_gas_budget(IOTA_IDENTITY_GAS_BUDGET)
        .build_and_execute(&identity_client)
        .await
        .map_err(|e| AppError::Error(format!("Failed to publish IOTA DID: {e}")))?
        .output;

    let identity = identity_client
        .get_identity(document.id().to_object_id())
        .await
        .map_err(|e| AppError::Error(format!("Failed to read published IOTA identity: {e}")))?;
    let identity_controller_cap = match identity {
        Identity::FullFledged(identity) => identity
            .get_controller_token(&identity_client)
            .await
            .map_err(|e| AppError::Error(format!("Failed to read identity controller cap: {e}")))?
            .map(|token| token.id().to_string()),
        Identity::Legacy(_) => None,
    };

    wallet.did = Some(document.id().to_string());
    wallet.did_network = Some(wallet.network);
    wallet.identity_controller_cap = identity_controller_cap;
    save_stored_wallet(&state, &wallet).await?;

    Ok(AppState {
        iota_wallet: IotaWalletState::from(&wallet),
        ..state
    })
}
