use std::sync::Arc;

use crate::{
    error::AppError,
    state::{
        actions::{listen, Action},
        iota_wallet::{
            actions::sign_prepared_transaction::SignPreparedIotaTransaction,
            reducers::sign_prepared_transaction::sign_prepared_transaction,
        },
        qr_code::actions::qrcode_scanned::QrCodeScanned,
        AppState,
    },
};

use serde::Deserialize;

#[derive(Deserialize)]
struct PreparedIotaTransactionQr {
    #[serde(rename = "type")]
    kind: String,
    tx_data_bcs_base64: String,
    #[serde(default = "default_submit")]
    submit: bool,
}

fn default_submit() -> bool {
    true
}

pub async fn read_prepared_iota_transaction(state: AppState, action: Action) -> Result<AppState, AppError> {
    let Some(scanned) = listen::<QrCodeScanned>(action) else {
        return Ok(state);
    };

    let qr_content = if is_prepared_iota_transaction_url(&scanned.form_urlencoded) {
        reqwest::get(&scanned.form_urlencoded)
            .await
            .map_err(|e| AppError::Error(format!("Failed to fetch prepared IOTA transaction: {e}")))?
            .text()
            .await
            .map_err(|e| AppError::Error(format!("Failed to read prepared IOTA transaction response: {e}")))?
    } else {
        scanned.form_urlencoded
    };

    let Ok(qr) = serde_json::from_str::<PreparedIotaTransactionQr>(&qr_content) else {
        return Ok(state);
    };

    if qr.kind != "iota:prepared-transaction" {
        return Ok(state);
    }

    sign_prepared_transaction(
        state,
        Arc::new(SignPreparedIotaTransaction {
            tx_data_bcs_base64: qr.tx_data_bcs_base64,
            submit: qr.submit,
        }),
    )
    .await
}

fn is_prepared_iota_transaction_url(value: &str) -> bool {
    (value.starts_with("https://") || value.starts_with("http://")) && value.contains("/payload/")
}
