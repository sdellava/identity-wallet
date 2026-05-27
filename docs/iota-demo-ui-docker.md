# IOTA demo UI Docker deploy

This compose publishes the QR transaction demo UI behind the existing external
Traefik network.

```powershell
$env:IOTA_TX_DEMO_HOST="iota-tx-demo.objectid.io"
docker compose -f docker-compose.iota-demo.yml up -d --build
```

The demo accepts a wallet address, prepares a tiny IOTA testnet transaction with
the IOTA TypeScript SDK, stores the prepared payload in memory, and renders an
inline SVG QR code containing a short `/payload/{id}` URL that UniMe can scan.
The payload endpoint returns:

```json
{
  "type": "iota:prepared-transaction",
  "network": "testnet",
  "submit": true,
  "tx_data_bcs_base64": "..."
}
```

The CORS middleware currently allows UniCore from:

- `http://unicore.objectid.io`
- `https://unicore.objectid.io`

Change `IOTA_TX_DEMO_HOST` in the shell or in an `.env` file next to the compose
file before deploying to a different host.

Optional runtime variables:

- `IOTA_TEST_TX_RPC_URL`, defaulting to the SDK testnet fullnode URL.
- `IOTA_TEST_TX_FAUCET_URL`, defaulting to `https://faucet.testnet.iota.cafe`.
- `IOTA_TEST_TX_GAS_BUDGET`, defaulting to `5000000`.
- `IOTA_TEST_TX_TRANSFER_AMOUNT`, defaulting to `1`.
