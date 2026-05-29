use crate::{
    error::AppError,
    state::{
        actions::{listen, Action},
        qr_code::actions::qrcode_scanned::QrCodeScanned,
        user_prompt::CurrentUserPrompt,
        AppState,
    },
};

use base64::{
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD},
    Engine as _,
};
use log::{info, warn};
use serde::Deserialize;
use serde_json::Value;
use url::Url;

const DEFAULT_WALLET_LOGIN_BASE_URL: &str = "https://server.objectid.io";

#[derive(Debug, Deserialize)]
struct WalletLoginQr {
    #[serde(rename = "type")]
    kind: Option<String>,
    #[serde(default, alias = "sessionId")]
    session_id: Option<String>,
    #[serde(default, alias = "requestUrl")]
    request_url: Option<String>,
    #[serde(default, alias = "requestUri", alias = "request_uri")]
    request_uri: Option<String>,
    #[serde(default, alias = "responseUrl", alias = "responseUri", alias = "response_uri")]
    response_url: Option<String>,
    #[serde(default, alias = "baseUrl", alias = "bridgeUrl", alias = "bridge_url")]
    base_url: Option<String>,
    #[serde(
        default,
        alias = "clientName",
        alias = "appName",
        alias = "app_name",
        alias = "dappName"
    )]
    client_name: Option<String>,
    #[serde(default)]
    origin: Option<String>,
    #[serde(default)]
    encryption: Option<WalletLoginEncryption>,
}

#[derive(Debug, Clone, Deserialize)]
struct WalletLoginEncryption {
    #[serde(default, alias = "publicKeyJwk")]
    public_key_jwk: Option<WalletLoginPublicKeyJwk>,
}

#[derive(Debug, Clone, Deserialize)]
struct WalletLoginPublicKeyJwk {
    x: String,
    y: String,
}

#[derive(Debug, Clone)]
struct WalletLoginRequest {
    session_id: String,
    request_url: String,
    response_url: String,
    origin: String,
    client_name: String,
    encryption_public_key_x: Option<String>,
    encryption_public_key_y: Option<String>,
}

pub async fn read_wallet_login_request(state: AppState, action: Action) -> Result<AppState, AppError> {
    let Some(scanned) = listen::<QrCodeScanned>(action) else {
        return Ok(state);
    };

    let Some(mut request) = parse_wallet_login_request(&scanned.form_urlencoded)? else {
        return Ok(state);
    };

    if let Ok(session) = fetch_wallet_login_session(&request.request_url).await {
        if let Some(client_name) = extract_client_name(&session) {
            request.client_name = client_name;
        }
    } else {
        warn!("Unable to fetch wallet login session metadata");
    }

    info!("wallet login request: {request:?}");

    Ok(AppState {
        current_user_prompt: Some(CurrentUserPrompt::WalletLogin {
            client_name: request.client_name,
            session_id: request.session_id,
            request_url: request.request_url,
            response_url: request.response_url,
            origin: request.origin,
            encryption_public_key_x: request.encryption_public_key_x,
            encryption_public_key_y: request.encryption_public_key_y,
        }),
        ..state
    })
}

pub(crate) fn is_wallet_login_request(value: &str) -> bool {
    parse_wallet_login_request(value).ok().flatten().is_some()
}

fn parse_wallet_login_request(value: &str) -> Result<Option<WalletLoginRequest>, AppError> {
    if let Ok(url) = Url::parse(value) {
        if let Some(request) = wallet_login_from_url(&url)? {
            return Ok(Some(request));
        }
    }

    if let Ok(qr) = serde_json::from_str::<WalletLoginQr>(value) {
        if let Some(request) = wallet_login_from_json(qr)? {
            return Ok(Some(request));
        }
    }

    Ok(None)
}

fn wallet_login_from_url(url: &Url) -> Result<Option<WalletLoginRequest>, AppError> {
    if matches!(url.scheme(), "objectid-wallet-login" | "objectid") {
        let Some(encoded_request) = url
            .query_pairs()
            .find_map(|(key, value)| (key == "request").then(|| value.into_owned()))
        else {
            return Ok(None);
        };

        let decoded_request = decode_wallet_login_request(&encoded_request)?;
        let qr = serde_json::from_str::<WalletLoginQr>(&decoded_request)
            .map_err(|_| AppError::InvalidQRCodeError(decoded_request))?;
        return wallet_login_from_json(qr);
    }

    if !matches!(url.scheme(), "https" | "http") {
        return Ok(None);
    }

    let path_segments = url.path_segments().map(|segments| segments.collect::<Vec<_>>());
    let Some(segments) = path_segments else {
        return Ok(None);
    };

    let Some(wallet_login_index) = segments.iter().position(|segment| *segment == "wallet-login") else {
        return Ok(None);
    };

    if segments.get(wallet_login_index + 1) != Some(&"sessions") {
        return Ok(None);
    }

    let Some(session_id) = segments
        .get(wallet_login_index + 2)
        .filter(|session_id| !session_id.is_empty())
    else {
        return Err(AppError::InvalidQRCodeError(url.to_string()));
    };

    let base_url = url[..url::Position::BeforePath].trim_end_matches('/').to_string();
    Ok(Some(build_request(
        &base_url,
        session_id,
        Some(url.to_string()),
        None,
        None,
        None,
        None,
    )?))
}

fn wallet_login_from_json(qr: WalletLoginQr) -> Result<Option<WalletLoginRequest>, AppError> {
    let is_wallet_login = qr
        .kind
        .as_deref()
        .map(|kind| {
            matches!(
                kind,
                "wallet-login" | "objectid:wallet-login" | "objectid-wallet-login" | "objectid.wallet-login.request"
            )
        })
        .unwrap_or_else(|| {
            qr.session_id.is_some() || qr.request_url.is_some() || qr.request_uri.is_some() || qr.response_url.is_some()
        });

    if !is_wallet_login {
        return Ok(None);
    }

    let request_url = qr.request_url.or(qr.request_uri);
    let session_id = qr
        .session_id
        .or_else(|| request_url.as_deref().and_then(extract_session_id_from_url))
        .or_else(|| qr.response_url.as_deref().and_then(extract_session_id_from_url))
        .ok_or_else(|| AppError::InvalidQRCodeError("Missing wallet login session id".to_string()))?;

    let base_url = qr.base_url.as_deref().unwrap_or(DEFAULT_WALLET_LOGIN_BASE_URL);
    let encryption_public_key = qr.encryption.and_then(|encryption| encryption.public_key_jwk);
    Ok(Some(build_request(
        base_url,
        &session_id,
        request_url,
        qr.response_url,
        qr.client_name,
        qr.origin,
        encryption_public_key,
    )?))
}

fn build_request(
    base_url: &str,
    session_id: &str,
    request_url: Option<String>,
    response_url: Option<String>,
    client_name: Option<String>,
    origin: Option<String>,
    encryption_public_key: Option<WalletLoginPublicKeyJwk>,
) -> Result<WalletLoginRequest, AppError> {
    let base = Url::parse(base_url).map_err(|_| AppError::InvalidQRCodeError(base_url.to_string()))?;
    let origin = origin.unwrap_or_else(|| base.origin().ascii_serialization());
    let normalized_base = base.as_str().trim_end_matches('/');
    let request_url = request_url.unwrap_or_else(|| format!("{normalized_base}/wallet-login/sessions/{session_id}"));
    let response_url =
        response_url.unwrap_or_else(|| format!("{normalized_base}/wallet-login/sessions/{session_id}/response"));

    Ok(WalletLoginRequest {
        session_id: session_id.to_string(),
        request_url,
        response_url,
        origin,
        client_name: client_name.unwrap_or_else(|| "ObjectID dApp".to_string()),
        encryption_public_key_x: encryption_public_key.as_ref().map(|key| key.x.clone()),
        encryption_public_key_y: encryption_public_key.map(|key| key.y),
    })
}

fn decode_wallet_login_request(value: &str) -> Result<String, AppError> {
    let decoded = URL_SAFE_NO_PAD
        .decode(value)
        .or_else(|_| URL_SAFE.decode(value))
        .or_else(|_| STANDARD_NO_PAD.decode(value))
        .or_else(|_| STANDARD.decode(value))
        .map_err(|_| AppError::InvalidQRCodeError(value.to_string()))?;

    String::from_utf8(decoded).map_err(|_| AppError::InvalidQRCodeError(value.to_string()))
}

fn extract_session_id_from_url(value: &str) -> Option<String> {
    let url = Url::parse(value).ok()?;
    let segments = url.path_segments()?.collect::<Vec<_>>();
    let wallet_login_index = segments.iter().position(|segment| *segment == "wallet-login")?;
    if segments.get(wallet_login_index + 1) != Some(&"sessions") {
        return None;
    }
    segments.get(wallet_login_index + 2).map(|segment| segment.to_string())
}

async fn fetch_wallet_login_session(request_url: &str) -> Result<Value, reqwest::Error> {
    reqwest::get(request_url).await?.json::<Value>().await
}

fn extract_client_name(session: &Value) -> Option<String> {
    [
        "/clientName",
        "/client_name",
        "/dappName",
        "/dapp_name",
        "/name",
        "/session/clientName",
        "/session/client_name",
        "/session/dappName",
        "/session/dapp_name",
        "/session/name",
    ]
    .iter()
    .find_map(|pointer| session.pointer(pointer)?.as_str().map(ToString::to_string))
}
