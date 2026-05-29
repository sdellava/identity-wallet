use crate::{
    error::AppError,
    state::{
        actions::{listen, Action},
        iota_wallet::actions::submit_wallet_login::SubmitWalletLogin,
        user_prompt::CurrentUserPrompt,
        AppState,
    },
};

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use bip39::{Language, Mnemonic, Seed};
use p256::{
    ecdh::EphemeralSecret,
    elliptic_curve::{
        rand_core::{OsRng, RngCore},
        sec1::ToEncodedPoint,
    },
    PublicKey,
};
use serde_json::json;

pub async fn submit_wallet_login(state: AppState, action: Action) -> Result<AppState, AppError> {
    let Some(_) = listen::<SubmitWalletLogin>(action) else {
        return Ok(state);
    };

    let CurrentUserPrompt::WalletLogin {
        response_url,
        session_id,
        encryption_public_key_x,
        encryption_public_key_y,
        ..
    } = state
        .current_user_prompt
        .clone()
        .ok_or(AppError::MissingStateParameterError("wallet login prompt"))?
    else {
        return Err(AppError::MissingStateParameterError("wallet login prompt"));
    };

    let seed_phrase = state
        .iota_wallet
        .seed_phrase
        .clone()
        .ok_or(AppError::MissingStateParameterError("IOTA wallet seed"))?;
    let seed = seed_hex_from_seed_phrase(&seed_phrase)?;
    let did = state
        .iota_wallet
        .did
        .clone()
        .ok_or(AppError::MissingStateParameterError("IOTA identity DID"))?;
    let did_document = state.iota_wallet.did_document.clone();
    let address = state.iota_wallet.address.clone();
    let network = state.iota_wallet.network.as_str();

    let response_builder = reqwest::Client::new().post(&response_url);
    let response = if let (Some(public_key_x), Some(public_key_y)) = (encryption_public_key_x, encryption_public_key_y)
    {
        let wallet_login_response =
            encrypt_wallet_login_response(&session_id, &did, &seed, network, &public_key_x, &public_key_y)?;

        response_builder
            .header(reqwest::header::CONTENT_TYPE, "text/plain")
            .body(wallet_login_response)
            .send()
            .await
    } else {
        let payload = json!({
            "sessionId": session_id,
            "seed": seed,
            "seed_phrase": seed,
            "did": did,
            "did_document": did_document,
            "address": address,
            "network": network,
        });
        response_builder
            .header(reqwest::header::CONTENT_TYPE, "text/plain")
            .body(payload.to_string())
            .send()
            .await
    }
    .map_err(|e| AppError::Error(format!("Failed to send wallet login response: {e}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Error(format!(
            "Wallet login response rejected by server: {status} {body}"
        )));
    }

    Ok(AppState {
        current_user_prompt: Some(CurrentUserPrompt::Redirect {
            target: "me".to_string(),
        }),
        ..state
    })
}

fn seed_hex_from_seed_phrase(seed_phrase: &str) -> Result<String, AppError> {
    let trimmed = seed_phrase.trim();
    if (trimmed.len() == 64 || trimmed.len() == 128) && trimmed.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Ok(trimmed.to_ascii_lowercase());
    }

    let mnemonic = Mnemonic::from_phrase(trimmed, Language::English)
        .map_err(|e| AppError::Error(format!("Failed to parse IOTA wallet mnemonic: {e:?}")))?;
    let seed = Seed::new(&mnemonic, "");
    Ok(hex_lower(seed.as_bytes()))
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn encrypt_wallet_login_response(
    session_id: &str,
    did: &str,
    seed: &str,
    network: &str,
    peer_public_key_x: &str,
    peer_public_key_y: &str,
) -> Result<String, AppError> {
    let peer_public_key = decode_p256_public_key(peer_public_key_x, peer_public_key_y)?;

    let secret = EphemeralSecret::random(&mut OsRng);
    let public_key = secret.public_key();
    let shared_secret = secret.diffie_hellman(&peer_public_key);
    let cipher = Aes256Gcm::new_from_slice(shared_secret.raw_secret_bytes().as_slice())
        .map_err(|_| AppError::Error("Failed to create wallet login cipher".to_string()))?;

    let mut iv = [0_u8; 12];
    OsRng.fill_bytes(&mut iv);

    let credentials = json!({
        "did": did,
        "seed": seed,
        "network": network,
    });
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&iv), credentials.to_string().as_bytes())
        .map_err(|_| AppError::Error("Failed to encrypt wallet login response".to_string()))?;

    let public_key_point = public_key.to_encoded_point(false);
    let public_key_x = public_key_point
        .x()
        .ok_or_else(|| AppError::Error("Missing wallet login public key x coordinate".to_string()))?;
    let public_key_y = public_key_point
        .y()
        .ok_or_else(|| AppError::Error("Missing wallet login public key y coordinate".to_string()))?;

    let response = json!({
        "type": "objectid.wallet-login.response",
        "version": 1,
        "sessionId": session_id,
        "encryption": {
            "alg": "ECDH-ES+A256GCM",
            "crv": "P-256",
            "publicKeyJwk": {
                "kty": "EC",
                "crv": "P-256",
                "ext": true,
                "key_ops": [],
                "x": URL_SAFE_NO_PAD.encode(public_key_x),
                "y": URL_SAFE_NO_PAD.encode(public_key_y),
            },
            "iv": URL_SAFE_NO_PAD.encode(iv),
            "ciphertext": URL_SAFE_NO_PAD.encode(ciphertext),
        },
    });

    let response_bytes = serde_json::to_vec(&response)
        .map_err(|e| AppError::Error(format!("Failed to serialize wallet login response: {e}")))?;
    Ok(URL_SAFE_NO_PAD.encode(response_bytes))
}

fn decode_p256_public_key(x: &str, y: &str) -> Result<PublicKey, AppError> {
    let x = URL_SAFE_NO_PAD
        .decode(x)
        .map_err(|_| AppError::Error("Invalid wallet login public key x coordinate".to_string()))?;
    let y = URL_SAFE_NO_PAD
        .decode(y)
        .map_err(|_| AppError::Error("Invalid wallet login public key y coordinate".to_string()))?;

    if x.len() != 32 || y.len() != 32 {
        return Err(AppError::Error(
            "Invalid wallet login P-256 public key length".to_string(),
        ));
    }

    let mut sec1 = Vec::with_capacity(65);
    sec1.push(0x04);
    sec1.extend_from_slice(&x);
    sec1.extend_from_slice(&y);

    PublicKey::from_sec1_bytes(&sec1).map_err(|_| AppError::Error("Invalid wallet login P-256 public key".to_string()))
}
