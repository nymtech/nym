import { setupMixTunnel, MixWebSocket } from '@nymproject/mix-websocket';

function log(line: string) {
  const el = document.getElementById('output') as HTMLPreElement;
  el.appendChild(document.createTextNode(`${line}\n`));
  el.scrollTop = el.scrollHeight;
}

async function main() {
  log('Setting up mixnet tunnel...');
  await setupMixTunnel();
  log('Tunnel ready.');

  log('Connecting to wss://echo.websocket.events...');
  const ws = new MixWebSocket('wss://echo.websocket.events');

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
