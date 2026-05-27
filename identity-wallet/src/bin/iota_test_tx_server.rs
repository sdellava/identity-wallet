use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    str::FromStr,
    sync::{Mutex, OnceLock},
    time::Duration,
};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use iota_sdk::{
    types::{
        base_types::IotaAddress, programmable_transaction_builder::ProgrammableTransactionBuilder,
        transaction::TransactionData,
    },
    IotaClient, IotaClientBuilder,
};
use serde_json::json;
use uuid::Uuid;

const FAUCET_URL: &str = "https://faucet.testnet.iota.cafe";
static PAYLOADS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let bind = std::env::var("IOTA_TEST_TX_SERVER_BIND").unwrap_or_else(|_| "127.0.0.1:8787".to_string());
    let listener = TcpListener::bind(&bind)?;
    println!("IOTA test tx server: http://{bind}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                tokio::spawn(async move {
                    if let Err(err) = handle(stream).await {
                        eprintln!("request failed: {err}");
                    }
                });
            }
            Err(err) => eprintln!("connection failed: {err}"),
        }
    }

    Ok(())
}

async fn handle(mut stream: TcpStream) -> anyhow::Result<()> {
    let mut buffer = [0; 4096];
    let n = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..n]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");

    if path.starts_with("/prepare?") {
        let address = query_value(path, "address").ok_or_else(|| anyhow::anyhow!("missing address"))?;
        let tx_payload = prepare_payload(&address).await?;
        let payload_id = Uuid::new_v4().to_string();
        PAYLOADS
            .get_or_init(Default::default)
            .lock()
            .map_err(|_| anyhow::anyhow!("payload store lock poisoned"))?
            .insert(payload_id.clone(), tx_payload.clone());

        let host = request_header(&request, "host").unwrap_or_else(|| "localhost:8787".to_string());
        let scheme = request_header(&request, "x-forwarded-proto").unwrap_or_else(|| "https".to_string());
        let payload_url = format!("{scheme}://{host}/payload/{payload_id}");

        respond_html(&mut stream, &qr_page(&address, &payload_url, &tx_payload))?;
    } else if let Some(payload_id) = path.strip_prefix("/payload/") {
        let payload = PAYLOADS
            .get_or_init(Default::default)
            .lock()
            .map_err(|_| anyhow::anyhow!("payload store lock poisoned"))?
            .get(payload_id)
            .cloned();

        match payload {
            Some(payload) => respond_json(&mut stream, 200, &payload)?,
            None => respond_json(
                &mut stream,
                404,
                &json!({ "error": "Prepared transaction payload not found" }).to_string(),
            )?,
        }
    } else {
        respond_html(&mut stream, form_page())?;
    }

    Ok(())
}

async fn prepare_payload(address: &str) -> anyhow::Result<String> {
    let sender = IotaAddress::from_str(address)?;
    let client = IotaClientBuilder::default().build_testnet().await?;
    let coin = match first_coin(&client, sender).await? {
        Some(coin) => coin,
        None => {
            request_tokens_from_faucet(sender).await?;
            wait_for_coin(&client, sender).await?
        }
    };

    let mut builder = ProgrammableTransactionBuilder::new();
    builder.pay_iota(vec![sender], vec![1])?;
    let programmable = builder.finish();
    let gas_price = client.read_api().get_reference_gas_price().await?;
    let tx_data =
        TransactionData::new_programmable(sender, vec![coin.object_ref()], programmable, 5_000_000, gas_price);
    let tx_data_bcs_base64 = STANDARD.encode(bcs::to_bytes(&tx_data)?);

    Ok(json!({
        "type": "iota:prepared-transaction",
        "network": "testnet",
        "submit": true,
        "tx_data_bcs_base64": tx_data_bcs_base64
    })
    .to_string())
}

async fn first_coin(client: &IotaClient, sender: IotaAddress) -> anyhow::Result<Option<iota_sdk::rpc_types::Coin>> {
    Ok(client
        .coin_read_api()
        .get_coins(sender, None, None, None)
        .await?
        .data
        .into_iter()
        .find(|coin| coin.balance >= 5_000_000))
}

async fn wait_for_coin(client: &IotaClient, sender: IotaAddress) -> anyhow::Result<iota_sdk::rpc_types::Coin> {
    for _ in 0..60 {
        if let Some(coin) = first_coin(client, sender).await? {
            return Ok(coin);
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    anyhow::bail!("faucet did not fund {sender} within 60 seconds")
}

async fn request_tokens_from_faucet(address: IotaAddress) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let response: serde_json::Value = client
        .post(format!("{FAUCET_URL}/v1/gas"))
        .json(&json!([{ "FixedAmountRequest": { "recipient": address.to_string() } }]))
        .send()
        .await?
        .json()
        .await?;
    let task = response
        .get("task")
        .and_then(|task| task.as_str())
        .ok_or_else(|| anyhow::anyhow!("faucet response did not include a task id: {response}"))?;

    for _ in 0..60 {
        let text = client
            .get(format!("{FAUCET_URL}/v1/status/{task}"))
            .send()
            .await?
            .text()
            .await?;
        if text.contains("SUCCEEDED") {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    anyhow::bail!("faucet task {task} did not complete within 60 seconds")
}

fn query_value(path: &str, key: &str) -> Option<String> {
    path.split_once('?')?.1.split('&').find_map(|pair| {
        let (candidate, value) = pair.split_once('=')?;
        (candidate == key).then(|| value.replace("%3A", ":"))
    })
}

fn request_header(request: &str, name: &str) -> Option<String> {
    let name = name.to_ascii_lowercase();
    request.lines().find_map(|line| {
        let (candidate, value) = line.split_once(':')?;
        (candidate.trim().eq_ignore_ascii_case(&name)).then(|| value.trim().to_string())
    })
}

fn respond_html(stream: &mut TcpStream, body: &str) -> anyhow::Result<()> {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {}\r\n\r\n{}",
        body.len(),
        body
    )?;
    Ok(())
}

fn respond_json(stream: &mut TcpStream, status: u16, body: &str) -> anyhow::Result<()> {
    let reason = match status {
        200 => "OK",
        404 => "Not Found",
        _ => "OK",
    };

    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json; charset=utf-8\r\naccess-control-allow-origin: *\r\ncontent-length: {}\r\n\r\n{}",
        body.len(),
        body
    )?;
    Ok(())
}

fn form_page() -> &'static str {
    r#"<!doctype html>
<html>
  <head><title>IOTA test tx QR</title></head>
  <body style="font-family: sans-serif; max-width: 720px; margin: 48px auto;">
    <h1>IOTA test tx QR</h1>
    <form action="/prepare">
      <label>Wallet address</label>
      <input name="address" style="display:block;width:100%;padding:12px;margin:8px 0 16px;font-family:monospace" />
      <button style="padding:12px 16px">Prepare transaction</button>
    </form>
  </body>
</html>"#
}

fn qr_page(address: &str, payload_url: &str, payload: &str) -> String {
    format!(
        r#"<!doctype html>
<html>
  <head><title>IOTA prepared tx</title></head>
  <body style="font-family: sans-serif; max-width: 760px; margin: 48px auto;">
    <h1>Prepared IOTA testnet transaction</h1>
    <p>Address: <code>{address}</code></p>
    <p>Scan this QR with UniMe:</p>
    <canvas id="qr" style="display:block;width:360px;height:360px;margin:24px 0;"></canvas>
    <p>Payload URL: <a href="{payload_url}">{payload_url}</a></p>
    <pre style="white-space: pre-wrap; word-break: break-all; background: #f5f5f5; padding: 16px;">{payload}</pre>
    <script src="https://cdn.jsdelivr.net/npm/qrcode@1.5.4/build/qrcode.min.js"></script>
    <script>
      QRCode.toCanvas(document.getElementById('qr'), {payload_url:?}, {{ width: 360 }}, function (error) {{
        if (error) document.body.insertAdjacentHTML('afterbegin', '<p style="color:#b91c1c">QR rendering failed: ' + error.message + '</p>');
      }});
    </script>
  </body>
</html>"#
    )
}
