# IOTA demo UI Docker deploy

This compose publishes the QR transaction demo UI behind the existing external
Traefik network.

```powershell
$env:IOTA_TX_DEMO_HOST="iota-tx-demo.objectid.io"
docker compose -f docker-compose.iota-demo.yml up -d --build
```

The demo accepts a wallet address, prepares a tiny IOTA testnet transaction,
and renders a QR payload that UniMe can scan:

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
