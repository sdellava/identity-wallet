use crate::{
    error::AppError,
    state::{
        actions::{listen, Action},
        iota_wallet::{
            actions::create_identity::CreateIotaIdentity,
            reducers::create_or_load_wallet::{load_stored_wallet, save_stored_wallet},
            IotaNetwork, IotaWalletState,
        },
        AppState,
    },
};

use identity_iota::{
    core::Timestamp,
    iota::{
        rebased::{client::IdentityClient, migration::Identity},
        IotaDID, IotaDocument,
    },
    iota_interaction::{
        types::{
            base_types::IotaAddress,
            crypto::{PublicKey, Signature},
            transaction::TransactionData,
        },
        IotaKeySignature, OptionalSend,
    },
    verification::{jws::JwsAlgorithm, MethodScope},
};
use identity_storage::{JwkDocumentExt, JwkMemStore, KeyIdMemstore, Storage};
use iota_keys::keystore::{AccountKeystore, InMemKeystore};
use iota_sdk::{types::crypto::SignatureScheme, IotaClientBuilder};
use iota_sdk_types::crypto::Intent;
use product_common::{
    gas_station::GasStationOptions,
    http_client::{HeaderMap, HttpClient, Method, Request, Response},
    transaction::{
        transaction_builder::{Transaction, TransactionBuilder},
        TransactionOutput,
    },
};
use secret_storage::{Error as SecretStorageError, Signer};
use std::future::Future;
use std::time::Duration;
use tokio::time::timeout;

const IOTA_IDENTITY_GAS_BUDGET: u64 = 50_000_000;
const GAS_STATION_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(45);

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

#[derive(Clone)]
struct ReqwestGasStationHttpClient(reqwest::Client);

impl Default for ReqwestGasStationHttpClient {
    fn default() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(75))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self(client)
    }
}

#[async_trait::async_trait]
impl HttpClient for ReqwestGasStationHttpClient {
    type Error = anyhow::Error;

    async fn send(&self, request: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, Self::Error> {
        let method = match request.method {
            Method::Get => reqwest::Method::GET,
            Method::Head => reqwest::Method::HEAD,
            Method::Post => reqwest::Method::POST,
            Method::Put => reqwest::Method::PUT,
            Method::Delete => reqwest::Method::DELETE,
            Method::Connect => reqwest::Method::CONNECT,
            Method::Options => reqwest::Method::OPTIONS,
            Method::Trace => reqwest::Method::TRACE,
            Method::Patch => reqwest::Method::PATCH,
        };

        let response = self
            .0
            .request(method, request.url.as_str())
            .headers(to_reqwest_headers(request.headers)?)
            .body(request.payload)
            .send()
            .await?;
        let status_code = response.status().as_u16();
        let headers = from_reqwest_headers(response.headers())?;
        let payload = response.bytes().await.unwrap_or_default().to_vec();

        Ok(Response {
            status_code,
            headers,
            payload,
        })
    }
}

fn to_reqwest_headers(headers: HeaderMap) -> anyhow::Result<reqwest::header::HeaderMap> {
    let mut map = reqwest::header::HeaderMap::with_capacity(headers.len());
    for (name, values) in headers {
        let name = reqwest::header::HeaderName::try_from(name)?;
        for value in values {
            map.append(&name, reqwest::header::HeaderValue::try_from(value)?);
        }
    }
    Ok(map)
}

fn from_reqwest_headers(headers: &reqwest::header::HeaderMap) -> anyhow::Result<HeaderMap> {
    let mut map = HeaderMap::default();
    for name in headers.keys() {
        let mut values = Vec::new();
        for value in headers.get_all(name) {
            values.push(value.to_str()?.to_owned());
        }
        map.insert(name.to_string(), values);
    }
    Ok(map)
}

pub(crate) struct WalletSigner {
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

pub(crate) async fn identity_client_for_wallet(
    wallet: &crate::state::iota_wallet::StoredIotaWallet,
) -> Result<IdentityClient<WalletSigner>, AppError> {
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
        return Err(AppError::Error(
            "Stored IOTA mnemonic does not match the stored address.".to_string(),
        ));
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

    IdentityClient::from_iota_client(iota_client, None)
        .await
        .map_err(|e| AppError::Error(format!("Failed to create IOTA Identity client: {e}")))?
        .with_signer(signer)
        .await
        .map_err(|e| AppError::Error(format!("Failed to attach IOTA Identity signer: {e}")))
}

async fn ensure_identity_document_has_controller_and_key(document: &mut IotaDocument) -> Result<(), AppError> {
    if document.controller().next().is_none() {
        document.set_controller([document.id().clone()]);
    }

    if document.methods(Some(MethodScope::VerificationMethod)).is_empty() {
        let storage = Storage::new(JwkMemStore::new(), KeyIdMemstore::new());
        document
            .generate_method(
                &storage,
                JwkMemStore::ED25519_KEY_TYPE,
                JwsAlgorithm::EdDSA,
                Some("key-0"),
                MethodScope::VerificationMethod,
            )
            .await
            .map_err(|e| AppError::Error(format!("Failed to generate IOTA identity key: {e}")))?;
    }

    document.metadata.updated = Some(Timestamp::now_utc());
    Ok(())
}

pub(crate) async fn execute_with_wallet_gas_station<Tx, F, Fut>(
    wallet: &crate::state::iota_wallet::StoredIotaWallet,
    identity_client: &IdentityClient<WalletSigner>,
    mut build_transaction: F,
) -> Result<TransactionOutput<Tx::Output>, AppError>
where
    Tx: Transaction + OptionalSend,
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<TransactionBuilder<Tx>, AppError>>,
{
    let mut last_error = None;
    let http_client = ReqwestGasStationHttpClient::default();

    for gas_station in gas_stations_for_network(wallet.network) {
        let transaction = build_transaction()
            .await
            .map_err(|e| AppError::Error(format!("Failed to prepare IOTA transaction: {e}")))?
            .with_gas_budget(IOTA_IDENTITY_GAS_BUDGET);
        let execution = transaction.execute_with_gas_station(
            identity_client,
            gas_station.url,
            &http_client,
            Some(GasStationOptions::default().with_auth_token(gas_station.token)),
        );

        match timeout(GAS_STATION_ATTEMPT_TIMEOUT, execution).await {
            Ok(Ok(output)) => return Ok(output),
            Ok(Err(error)) => last_error = Some(format!("{} failed: {error}", gas_station.url)),
            Err(_) => last_error = Some(format!("{} timed out", gas_station.url)),
        }
    }

    Err(AppError::Error(format!(
        "All configured IOTA gas stations failed. {}",
        last_error.unwrap_or_else(|| "No gas station was attempted.".to_string())
    )))
}

pub(crate) async fn update_did_document_with_gas_station(
    wallet: &crate::state::iota_wallet::StoredIotaWallet,
    identity_client: &IdentityClient<WalletSigner>,
    document: IotaDocument,
) -> Result<IotaDocument, AppError> {
    let mut identity = match identity_client
        .get_identity(document.id().to_object_id())
        .await
        .map_err(|e| AppError::Error(format!("Failed to read IOTA identity: {e}")))?
    {
        Identity::FullFledged(identity) => identity,
        Identity::Legacy(_) => return Err(AppError::Error("Only new IOTA identities can be updated.".to_string())),
    };
    let controller_token = identity
        .get_controller_token(identity_client)
        .await
        .map_err(|e| AppError::Error(format!("Failed to read identity controller token: {e}")))?
        .ok_or_else(|| AppError::Error("Wallet address is not a controller for this identity.".to_string()))?;

    let mut last_error = None;
    let http_client = ReqwestGasStationHttpClient::default();

    for gas_station in gas_stations_for_network(wallet.network) {
        let transaction = identity
            .update_did_document(document.clone(), &controller_token)
            .finish(identity_client)
            .await
            .map_err(|e| AppError::Error(format!("Failed to prepare DID document update: {e}")))
            .map(|transaction| transaction.with_gas_budget(IOTA_IDENTITY_GAS_BUDGET))?;

        let execution = transaction.execute_with_gas_station(
            identity_client,
            gas_station.url,
            &http_client,
            Some(GasStationOptions::default().with_auth_token(gas_station.token)),
        );

        match timeout(GAS_STATION_ATTEMPT_TIMEOUT, execution).await {
            Ok(Ok(_)) => return Ok(document),
            Ok(Err(error)) => last_error = Some(format!("{} failed: {error}", gas_station.url)),
            Err(_) => last_error = Some(format!("{} timed out", gas_station.url)),
        }
    }

    Err(AppError::Error(format!(
        "All configured IOTA gas stations failed. {}",
        last_error.unwrap_or_else(|| "No gas station was attempted.".to_string())
    )))
}

pub async fn create_identity(state: AppState, action: Action) -> Result<AppState, AppError> {
    if listen::<CreateIotaIdentity>(action).is_none() {
        return Ok(state);
    }

    let mut wallet = load_stored_wallet(&state).await?;
    let identity_client = match identity_client_for_wallet(&wallet).await {
        Ok(identity_client) => identity_client,
        Err(e) => {
            let last_error = e.to_string();
            return Ok(AppState {
                iota_wallet: IotaWalletState {
                    network: wallet.network,
                    address: Some(wallet.address),
                    public_key: wallet.public_key,
                    seed_phrase: Some(wallet.mnemonic),
                    did: wallet.did,
                    did_document: wallet.did_document,
                    identity_controller_cap: wallet.identity_controller_cap,
                    faucet_status: state.iota_wallet.faucet_status,
                    last_error: Some(last_error),
                    ..state.iota_wallet
                },
                ..state
            });
        }
    };

    let mut document = if let Some(did) = &wallet.did {
        let did = IotaDID::parse(did).map_err(|e| AppError::Error(format!("Stored IOTA DID is invalid: {e}")))?;
        match identity_client.resolve_did(&did).await {
            Ok(mut document) => {
                ensure_identity_document_has_controller_and_key(&mut document).await?;
                match update_did_document_with_gas_station(&wallet, &identity_client, document).await {
                    Ok(document) => document,
                    Err(e) => {
                        return Ok(AppState {
                            iota_wallet: IotaWalletState {
                                faucet_status: state.iota_wallet.faucet_status,
                                last_error: Some(format!("Failed to update IOTA DID document: {e}")),
                                ..IotaWalletState::from(&wallet)
                            },
                            ..state
                        });
                    }
                }
            }
            Err(e) => {
                return Ok(AppState {
                    iota_wallet: IotaWalletState {
                        faucet_status: state.iota_wallet.faucet_status,
                        last_error: Some(format!("Failed to resolve stored IOTA DID: {e}")),
                        ..IotaWalletState::from(&wallet)
                    },
                    ..state
                });
            }
        }
    } else {
        IotaDocument::new(identity_client.network())
    };

    if wallet.did.is_none() {
        ensure_identity_document_has_controller_and_key(&mut document).await?;
        document = match execute_with_wallet_gas_station(&wallet, &identity_client, || async {
            Ok(identity_client.publish_did_document(document.clone()))
        })
        .await
        {
            Ok(result) => result.output,
            Err(e) => {
                return Ok(AppState {
                    iota_wallet: IotaWalletState {
                        faucet_status: state.iota_wallet.faucet_status,
                        last_error: Some(format!("Failed to create IOTA identity: {e}")),
                        ..IotaWalletState::from(&wallet)
                    },
                    ..state
                });
            }
        };
    }

    let identity = match identity_client.get_identity(document.id().to_object_id()).await {
        Ok(identity) => identity,
        Err(e) => {
            return Ok(AppState {
                iota_wallet: IotaWalletState {
                    faucet_status: state.iota_wallet.faucet_status,
                    last_error: Some(format!("Failed to read created IOTA identity: {e}")),
                    ..IotaWalletState::from(&wallet)
                },
                ..state
            });
        }
    };
    let identity_controller_cap = match identity {
        Identity::FullFledged(identity) => match identity.get_controller_token(&identity_client).await {
            Ok(token) => token.map(|token| token.id().to_string()),
            Err(e) => {
                return Ok(AppState {
                    iota_wallet: IotaWalletState {
                        faucet_status: state.iota_wallet.faucet_status,
                        last_error: Some(format!("Failed to read identity controller cap: {e}")),
                        ..IotaWalletState::from(&wallet)
                    },
                    ..state
                });
            }
        },
        Identity::Legacy(_) => None,
    };

    wallet.did = Some(document.id().to_string());
    wallet.did_network = Some(wallet.network);
    wallet.did_document = serde_json::to_string_pretty(&document).ok();
    wallet.identity_controller_cap = identity_controller_cap;
    save_stored_wallet(&state, &wallet).await?;

    Ok(AppState {
        iota_wallet: IotaWalletState::from(&wallet),
        ..state
    })
}
