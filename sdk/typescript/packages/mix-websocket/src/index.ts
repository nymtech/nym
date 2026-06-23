// @nymproject/mix-websocket
//
// WebSocket-like class for sending WS/WSS traffic through the Nym mixnet.
// The exposed `MixWebSocket` mirrors the browser `WebSocket` API where it
// makes sense (open/message/close/error events, send/close methods).

import {
  getMixTunnel,
  setupMixTunnel,
  disconnectMixTunnel,
  getTunnelState,
  proxy,
  SetupMixTunnelOpts,
  WsEventType,
} from '@nymproject/mix-tunnel';

export { setupMixTunnel, disconnectMixTunnel, getTunnelState };
export type { SetupMixTunnelOpts };

export type MixWebSocketReadyState = 0 | 1 | 2 | 3; // CONNECTING | OPEN | CLOSING | CLOSED

const CONNECTING: MixWebSocketReadyState = 0;
const OPEN: MixWebSocketReadyState = 1;
const CLOSING: MixWebSocketReadyState = 2;
const CLOSED: MixWebSocketReadyState = 3;

/**
 * WebSocket-like channel over the Nym mixnet. The tunnel must already be
 * set up (`setupMixTunnel()`) before constructing one.
 *
 * Differences from the browser `WebSocket`:
 *   - Constructor resolves asynchronously; use `await ws.opened()` if you
 *     need to block until the upgrade completes.
 *   - `binaryType` is fixed to `arraybuffer` (no Blob support).
 *   - No `bufferedAmount`; the tunnel queues writes through the worker.
 */
export class MixWebSocket extends EventTarget {
  readonly url: string;
  readonly protocols: string[];

  private handleIdPromise: Promise<number>;
  private state: MixWebSocketReadyState = CONNECTING;

  constructor(url: string, protocols?: string | string[]) {
    super();
    this.url = url;
    const protocolList = Array.isArray(protocols) ? protocols : protocols ? [protocols] : [];
    this.protocols = protocolList;

    this.handleIdPromise = (async () => {
      const tunnel = await getMixTunnel();
      // Use mix-tunnel's `proxy` re-export so the marker Symbol matches the
      // Comlink instance that serialises the call. A locally-bundled Comlink
      // would mark with a different Symbol → fall-through to structured-clone
      // → "Function object could not be cloned" DOMException.
      const onEvent = proxy((_handle: number, type: WsEventType, data: unknown) => {
        this.handleEvent(type, data);
      });
      return tunnel.mixWebSocket(url, protocolList, onEvent);
    })();

    // If the upgrade fails before `open` fires, transition to CLOSED and
    // dispatch `error`. `opened()` listens for both events so callers don't hang.
    this.handleIdPromise.catch((err) => {
      this.state = CLOSED;
      // Surface the underlying cause to console so playground / consumer code
      // can diagnose; the standard `error` Event itself carries no payload.
      // eslint-disable-next-line no-console
      console.error('[MixWebSocket] connection failed:', err);
      const evt = new Event('error') as Event & { message?: string };
      evt.message = err instanceof Error ? err.message : String(err);
      this.dispatchEvent(evt);
    });
  }

  get readyState(): MixWebSocketReadyState {
    return this.state;
  }

  /**
   * Block until the WebSocket transitions out of `CONNECTING`. Resolves when
   * `open` fires (or when the connection fails before opening).
   */
  opened(): Promise<void> {
    if (this.state !== CONNECTING) return Promise.resolve();
    return new Promise((resolve) => {
      const listener = () => {
        this.removeEventListener('open', listener);
        this.removeEventListener('error', listener);
        resolve();
      };
      this.addEventListener('open', listener);
      this.addEventListener('error', listener);
    });
  }

  async send(data: string | ArrayBuffer | Uint8Array): Promise<void> {
    if (this.state !== OPEN) {
      throw new Error(`MixWebSocket.send: state is ${this.state}, expected OPEN`);
    }
    const tunnel = await getMixTunnel();
    const handleId = await this.handleIdPromise;
    await tunnel.wsSend(handleId, data);
  }

  async close(code = 1000, reason = ''): Promise<void> {
    if (this.state === CLOSING || this.state === CLOSED) return;
    this.state = CLOSING;
    const tunnel = await getMixTunnel();
    const handleId = await this.handleIdPromise;
    await tunnel.wsClose(handleId, code, reason);
  }

  private handleEvent(type: WsEventType, data: unknown) {
    switch (type) {
      case 'open':
        this.state = OPEN;
        this.dispatchEvent(new Event('open'));
        break;
      case 'text':
        this.dispatchEvent(new MessageEvent('message', { data }));
        break;
      case 'binary':
        // wasm hands us a Uint8Array view; surface as ArrayBuffer to match the
        // standard WebSocket(binaryType=arraybuffer) shape.
        this.dispatchEvent(new MessageEvent('message', { data: toArrayBuffer(data) }));
        break;
      case 'close':
        this.state = CLOSED;
        this.dispatchEvent(new CloseEvent('close', closeInit(data)));
        break;
      case 'error': {
        this.state = CLOSED;
        // Mirror the constructor's catch-handler shape: attach `.message`
        // (non-standard but consistent) so application code can read the
        // cause without scraping the worker's console.error output.
        // smolmix-wasm always fires `error` with the stringified Rust error
        // (see `mixwebsocket.rs` `fire_ws_event(..., "error", ...)`).
        const evt = new Event('error') as Event & { message?: string };
        evt.message = typeof data === 'string' ? data : String(data ?? '');
        this.dispatchEvent(evt);
        break;
      }
      default:
        // Unknown event type; ignore.
        break;
    }
  }
}

function toArrayBuffer(data: unknown): ArrayBuffer {
  if (data instanceof ArrayBuffer) return data;
  if (data instanceof Uint8Array) {
    return data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength) as ArrayBuffer;
  }
  return new ArrayBuffer(0);
}

function closeInit(data: unknown): CloseEventInit {
  if (data && typeof data === 'object') {
    const obj = data as { code?: number; reason?: string };
    return { code: obj.code, reason: obj.reason };
  }
  return {};
}
