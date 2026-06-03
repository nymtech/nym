// Live mix-websocket demo. Committed pre-publish and NOT yet imported by any
// page; see ./shared.tsx for why the import is dynamic and how to activate.

import React, { useRef, useState } from 'react';
import {
  DemoFrame,
  useLog,
  btnStyle,
  inputStyle,
  type MixWebSocketModule,
  type MixWebSocketLike,
} from './shared';

// Same echo endpoint the docs quick start and the Rust smolmix example use.
const WS_URL = 'wss://ws.postman-echo.com/raw';

export function MixWebSocketDemo() {
  const { lines, log } = useLog();
  const [message, setMessage] = useState('hello mixnet');
  const [connected, setConnected] = useState(false);
  const [busy, setBusy] = useState(false);
  const ws = useRef<MixWebSocketLike | null>(null);

  async function connect() {
    setBusy(true);
    try {
      log('Loading mix-websocket (wasm)...');
      // @ts-ignore -- @nymproject/mix-websocket is published separately; absent at build time pre-publish
      const mod = (await import('@nymproject/mix-websocket')) as unknown as MixWebSocketModule;
      log('Bringing up the mixnet tunnel...');
      await mod.setupMixTunnel();

      log(`Connecting to ${WS_URL}...`);
      const socket = new mod.MixWebSocket(WS_URL);
      socket.addEventListener('message', (e) => {
        const data = (e as MessageEvent).data;
        log(typeof data === 'string' ? `< ${data}` : `< binary: ${(data as ArrayBuffer).byteLength} bytes`);
      });
      socket.addEventListener('close', (e) => {
        const ce = e as CloseEvent;
        log(`< close: code=${ce.code} reason=${ce.reason || '(empty)'}`);
        setConnected(false);
      });
      socket.addEventListener('error', (e) => {
        log(`< error: ${(e as Event & { message?: string }).message ?? '(no detail)'}`);
      });

      await socket.opened();
      ws.current = socket;
      setConnected(true);
      log('Connected.');
    } catch (err) {
      log(`Error: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setBusy(false);
    }
  }

  async function send() {
    if (!ws.current || !message) return;
    log(`> ${message}`);
    await ws.current.send(message);
  }

  async function close() {
    if (!ws.current) return;
    await ws.current.close(1000, 'user requested');
    ws.current = null;
  }

  return (
    <DemoFrame title="MixWebSocket" lines={lines}>
      {!connected ? (
        <button style={btnStyle} onClick={connect} disabled={busy}>
          {busy ? 'Connecting...' : 'Connect'}
        </button>
      ) : (
        <>
          <input
            style={inputStyle}
            value={message}
            onChange={(e) => setMessage(e.target.value)}
            placeholder="message to echo"
          />
          <button style={btnStyle} onClick={send}>
            Send
          </button>
          <button style={btnStyle} onClick={close}>
            Close
          </button>
        </>
      )}
    </DemoFrame>
  );
}
