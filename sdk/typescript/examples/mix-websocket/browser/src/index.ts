import { setupMixTunnel, MixWebSocket, type SetupMixTunnelOpts } from '@nymproject/mix-websocket';

function log(line: string) {
  const el = document.getElementById('output') as HTMLPreElement;
  el.appendChild(document.createTextNode(`${line}\n`));
  el.scrollTop = el.scrollHeight;
}

// Tunnel configuration. Every field is optional.
//
// `debug: true` turns on smolmix-wasm's verbose tracing so you can watch
// the TLS handshake and WebSocket frame exchange in DevTools. Leave it off
// in production.
const setupOpts: SetupMixTunnelOpts = {
  debug: true,

  // Pin a specific exit IPR. Otherwise auto-discovered from the topology.
  // preferredIpr: 'D1rrUqJY9pesL3pTaMaxLnpZGGYQ4ZpZwpQXCqaeBXTW.6PpFkRvF...',

  // Anonymity / performance trade-off. Cover traffic + Poisson padding
  // smear timing patterns at the cost of bandwidth. Default: both on.
  // disableCoverTraffic: true,
  // disablePoissonTraffic: true,

  // TCP keepalive cadence for the underlying smoltcp socket. Default: 10s.
  // Lower it if you need quicker dead-peer detection on idle WebSockets.
  // tcpKeepaliveMs: 5_000,

  // Connect budget for the TCP + TLS + WS handshake. Default: 60s.
  // connectTimeoutMs: 30_000,
};

// Public echo server. Sends each frame back to the client.
const WS_URL = 'wss://echo.websocket.org';

async function main() {
  log('Setting up mixnet tunnel...');
  await setupMixTunnel(setupOpts);
  log('Tunnel ready.');

  log(`Connecting to ${WS_URL}...`);
  const ws = new MixWebSocket(WS_URL);

  ws.addEventListener('open', () => log('< open'));
  ws.addEventListener('message', (e) => {
    const data = (e as MessageEvent).data;
    if (typeof data === 'string') {
      log(`< text: ${data}`);
    } else if (data instanceof ArrayBuffer) {
      log(`< binary: ${data.byteLength} bytes`);
    }
  });
  ws.addEventListener('close', (e) => {
    const ce = e as CloseEvent;
    log(`< close: code=${ce.code} reason=${ce.reason || '(empty)'}`);
  });
  ws.addEventListener('error', (e) => {
    const evt = e as Event & { message?: string };
    log(`< error: ${evt.message ?? '(no detail)'}`);
  });

  await ws.opened();

  const sendBtn = document.getElementById('send') as HTMLButtonElement;
  const closeBtn = document.getElementById('close') as HTMLButtonElement;
  const input = document.getElementById('message') as HTMLInputElement;

  sendBtn.disabled = false;
  closeBtn.disabled = false;

  sendBtn.addEventListener('click', async () => {
    const value = input.value;
    if (!value) return;
    log(`> ${value}`);
    await ws.send(value);
  });

  closeBtn.addEventListener('click', async () => {
    sendBtn.disabled = true;
    closeBtn.disabled = true;
    await ws.close(1000, 'user requested');
  });
}

window.addEventListener('DOMContentLoaded', () => {
  main().catch((err) => {
    console.error(err);
    log(`Error: ${err instanceof Error ? err.message : String(err)}`);
  });
});
