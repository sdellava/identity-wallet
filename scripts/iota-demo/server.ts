import http, { type IncomingMessage, type ServerResponse } from 'node:http';
import { Buffer } from 'node:buffer';
import { randomUUID } from 'node:crypto';

import { IotaClient, getFullnodeUrl } from '@iota/iota-sdk/client';
import { Transaction } from '@iota/iota-sdk/transactions';
import { isValidIotaAddress, normalizeIotaAddress } from '@iota/iota-sdk/utils';
import QRCode from 'qrcode';

const DEFAULT_BIND = '127.0.0.1:8787';
const DEFAULT_NETWORK = 'testnet';
const DEFAULT_GAS_BUDGET = 5_000_000n;
const DEFAULT_TRANSFER_AMOUNT = 1n;

const payloads = new Map<string, string>();
const bind = process.env.IOTA_TEST_TX_SERVER_BIND ?? DEFAULT_BIND;
const network = process.env.IOTA_TEST_TX_NETWORK ?? DEFAULT_NETWORK;
const rpcUrl = process.env.IOTA_TEST_TX_RPC_URL ?? getFullnodeUrl(network as Parameters<typeof getFullnodeUrl>[0]);
const gasBudget = BigInt(process.env.IOTA_TEST_TX_GAS_BUDGET ?? DEFAULT_GAS_BUDGET.toString());
const transferAmount = BigInt(process.env.IOTA_TEST_TX_TRANSFER_AMOUNT ?? DEFAULT_TRANSFER_AMOUNT.toString());

const client = new IotaClient({ url: rpcUrl });
const { host, port } = parseBind(bind);

const server = http.createServer((request, response) => {
  void handleRequest(request, response).catch((error: unknown) => {
    console.error('request failed:', error);
    respondHtml(response, 500, errorPage(error));
  });
});

server.listen(port, host, () => {
  console.log(`IOTA test tx demo server: http://${host}:${port}`);
  console.log(`IOTA RPC: ${rpcUrl}`);
});

async function handleRequest(request: IncomingMessage, response: ServerResponse) {
  const url = new URL(request.url ?? '/', externalOrigin(request));

  if (request.method === 'OPTIONS') {
    respondOptions(response);
    return;
  }

  if (request.method !== 'GET') {
    respondJson(response, 405, { error: 'Method not allowed' });
    return;
  }

  if (url.pathname === '/prepare') {
    const address = url.searchParams.get('address')?.trim() ?? '';
    if (!isValidIotaAddress(address)) {
      respondHtml(response, 400, errorPage(`Invalid IOTA address: ${address || '(empty)'}`));
      return;
    }

    const normalizedAddress = normalizeIotaAddress(address);
    const payload = await preparePayload(normalizedAddress);
    const payloadId = randomUUID();
    payloads.set(payloadId, payload);

    const payloadUrl = new URL(`/payload/${payloadId}`, externalOrigin(request)).toString();
    respondHtml(response, 200, await qrPage(normalizedAddress, payloadUrl, payload));
    return;
  }

  if (url.pathname.startsWith('/payload/')) {
    const payloadId = url.pathname.slice('/payload/'.length);
    const payload = payloads.get(payloadId);
    if (payload) {
      respondRawJson(response, 200, payload);
    } else {
      respondJson(response, 404, { error: 'Prepared transaction payload not found' });
    }
    return;
  }

  respondHtml(response, 200, formPage());
}

async function preparePayload(address: string) {
  const transaction = new Transaction();
  transaction.setSender(address);
  transaction.setGasBudget(gasBudget);

  const [coin] = transaction.splitCoins(transaction.gas, [transferAmount]);
  transaction.transferObjects([coin], address);

  const gasPrice = await client.getReferenceGasPrice();
  const txBytes = await transaction.build({ client, onlyTransactionKind: true });
  return JSON.stringify({
    type: 'iota:prepared-transaction',
    network,
    submit: true,
    gas_budget: Number(gasBudget),
    gas_price: Number(gasPrice),
    tx_kind_bcs_base64: Buffer.from(txBytes).toString('base64'),
  });
}

async function qrPage(address: string, payloadUrl: string, payload: string) {
  const qrSvg = await QRCode.toString(payloadUrl, {
    type: 'svg',
    width: 360,
    margin: 2,
    errorCorrectionLevel: 'M',
  });

  return `<!doctype html>
<html>
  <head>
    <title>IOTA prepared tx</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
  </head>
  <body style="font-family: sans-serif; max-width: 760px; margin: 48px auto; padding: 0 16px;">
    <h1>Prepared IOTA testnet transaction</h1>
    <p>Address: <code>${htmlEscape(address)}</code></p>
    <p>Scan this QR with UniMe:</p>
    <div style="width:360px;max-width:100%;margin:24px 0;">${qrSvg}</div>
    <p>Payload URL: <a href="${htmlAttrEscape(payloadUrl)}">${htmlEscape(payloadUrl)}</a></p>
    <pre style="white-space: pre-wrap; word-break: break-all; background: #f5f5f5; padding: 16px;">${htmlEscape(payload)}</pre>
  </body>
</html>`;
}

function formPage() {
  return `<!doctype html>
<html>
  <head>
    <title>IOTA test tx QR</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
  </head>
  <body style="font-family: sans-serif; max-width: 720px; margin: 48px auto; padding: 0 16px;">
    <h1>IOTA test tx QR</h1>
    <form action="/prepare">
      <label for="address">Wallet address</label>
      <input id="address" name="address" style="display:block;width:100%;padding:12px;margin:8px 0 16px;font-family:monospace" />
      <button style="padding:12px 16px">Prepare transaction</button>
    </form>
  </body>
</html>`;
}

function errorPage(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  return `<!doctype html>
<html>
  <head><title>IOTA demo error</title></head>
  <body style="font-family: sans-serif; max-width: 720px; margin: 48px auto; padding: 0 16px;">
    <h1>IOTA demo error</h1>
    <p style="color:#b91c1c">${htmlEscape(message)}</p>
    <p><a href="/">Back</a></p>
  </body>
</html>`;
}

function respondOptions(response: ServerResponse) {
  response.writeHead(204, corsHeaders());
  response.end();
}

function respondHtml(response: ServerResponse, status: number, body: string) {
  response.writeHead(status, {
    ...corsHeaders(),
    'content-type': 'text/html; charset=utf-8',
    'content-length': Buffer.byteLength(body),
  });
  response.end(body);
}

function respondJson(response: ServerResponse, status: number, body: unknown) {
  respondRawJson(response, status, JSON.stringify(body));
}

function respondRawJson(response: ServerResponse, status: number, body: string) {
  response.writeHead(status, {
    ...corsHeaders(),
    'content-type': 'application/json; charset=utf-8',
    'content-length': Buffer.byteLength(body),
  });
  response.end(body);
}

function corsHeaders() {
  return {
    'access-control-allow-origin': '*',
    'access-control-allow-methods': 'GET,OPTIONS',
    'access-control-allow-headers': '*',
  };
}

function externalOrigin(request: IncomingMessage) {
  const hostHeader = request.headers.host ?? `localhost:${port}`;
  const protoHeader = request.headers['x-forwarded-proto'];
  const scheme = Array.isArray(protoHeader) ? protoHeader[0] : (protoHeader ?? 'https');
  return `${scheme}://${hostHeader}`;
}

function parseBind(value: string) {
  const separator = value.lastIndexOf(':');
  if (separator === -1) {
    return { host: value, port: 8787 };
  }

  return {
    host: value.slice(0, separator) || '0.0.0.0',
    port: Number.parseInt(value.slice(separator + 1), 10),
  };
}

function htmlEscape(value: string) {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}

function htmlAttrEscape(value: string) {
  return htmlEscape(value);
}
