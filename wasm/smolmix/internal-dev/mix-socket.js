// MixSocket — drop-in WebSocket replacement over the Nym mixnet.
//
// Mirrors the standard browser WebSocket API (RFC 6455):
//
//   const ws = new MixSocket('wss://echo.example.com/ws');
//   ws.onopen = () => ws.send('hello');
//   ws.onmessage = (e) => console.log(e.data);
//   ws.onclose = (e) => console.log(e.code, e.reason);
//
// Communicates with the worker via raw postMessage (not Comlink).
// The worker maps connId → WASM handleId and forwards events back.

const CONNECTING = 0;
const OPEN = 1;
const CLOSING = 2;
const CLOSED = 3;

let _worker = null;
let _nextConnId = 1;
const _instances = new Map();

function _onWorkerMessage(event) {
  const msg = event.data;
  if (msg?.kind !== 'ws-event') return;

  const instance = _instances.get(msg.connId);
  if (!instance) return;

  instance._handleEvent(msg.type, msg.data);
}

export class MixSocket extends EventTarget {
  static CONNECTING = CONNECTING;
  static OPEN = OPEN;
  static CLOSING = CLOSING;
  static CLOSED = CLOSED;

  /**
   * Bind the raw Worker so MixSocket can post messages to it.
   * Call once during app setup, after the worker emits 'Loaded'.
   */
  static _initWorker(worker) {
    _worker = worker;
    _worker.addEventListener('message', _onWorkerMessage);
  }

  /**
   * @param {string} url - WebSocket URL (ws:// or wss://)
   * @param {string|string[]} [protocols] - Sub-protocol(s) to negotiate
   */
  constructor(url, protocols) {
    super();

    if (!_worker) {
      throw new Error(
        'MixSocket: worker not initialised — call MixSocket._initWorker(worker) first',
      );
    }

    this._connId = _nextConnId++;
    this._url = url;
    this._readyState = CONNECTING;
    this._protocol = '';
    this._binaryType = 'blob';

    // Standard event handler properties
    this.onopen = null;
    this.onmessage = null;
    this.onclose = null;
    this.onerror = null;

    _instances.set(this._connId, this);

    const protoList = protocols
      ? typeof protocols === 'string'
        ? [protocols]
        : [...protocols]
      : [];

    _worker.postMessage({
      kind: 'ws-connect',
      connId: this._connId,
      url,
      protocols: protoList,
    });
  }

  get url() {
    return this._url;
  }
  get readyState() {
    return this._readyState;
  }
  get protocol() {
    return this._protocol;
  }
  get extensions() {
    return '';
  }
  get binaryType() {
    return this._binaryType;
  }
  set binaryType(val) {
    if (val === 'blob' || val === 'arraybuffer') this._binaryType = val;
  }
  get bufferedAmount() {
    return 0;
  }

  /**
   * Send data over the WebSocket.
   * @param {string|ArrayBuffer|ArrayBufferView} data
   */
  send(data) {
    if (this._readyState !== OPEN) {
      throw new DOMException('WebSocket is not open', 'InvalidStateError');
    }

    // Normalise typed arrays to Uint8Array for structured clone
    let payload = data;
    if (data instanceof ArrayBuffer) {
      payload = new Uint8Array(data);
    } else if (ArrayBuffer.isView(data) && !(data instanceof Uint8Array)) {
      payload = new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
    }

    _worker.postMessage({
      kind: 'ws-send',
      connId: this._connId,
      payload,
    });
  }

  /**
   * Initiate the closing handshake.
   * @param {number} [code=1000] - Status code
   * @param {string} [reason=''] - Human-readable reason
   */
  close(code = 1000, reason = '') {
    if (this._readyState === CLOSING || this._readyState === CLOSED) return;
    this._readyState = CLOSING;
    _worker.postMessage({
      kind: 'ws-close',
      connId: this._connId,
      code,
      reason,
    });
  }

  /** @internal Route an event from the worker to the appropriate handler. */
  _handleEvent(type, data) {
    switch (type) {
      case 'open': {
        this._readyState = OPEN;
        this._protocol = data || '';
        const ev = new Event('open');
        this.dispatchEvent(ev);
        if (this.onopen) this.onopen(ev);
        break;
      }

      case 'text': {
        const ev = new MessageEvent('message', { data });
        this.dispatchEvent(ev);
        if (this.onmessage) this.onmessage(ev);
        break;
      }

      case 'binary': {
        let payload;
        if (this._binaryType === 'arraybuffer') {
          payload = data instanceof Uint8Array ? data.buffer : data;
        } else {
          // Default: blob
          payload = data instanceof Uint8Array ? new Blob([data]) : data;
        }
        const ev = new MessageEvent('message', { data: payload });
        this.dispatchEvent(ev);
        if (this.onmessage) this.onmessage(ev);
        break;
      }

      case 'close': {
        this._readyState = CLOSED;
        _instances.delete(this._connId);

        // Parse close info: "1000 normal closure" → code=1000, reason="normal closure"
        let code = 1005;
        let reason = '';
        if (typeof data === 'string') {
          const match = data.match(/^(\d+)\s*(.*)/);
          if (match) {
            code = parseInt(match[1], 10);
            reason = match[2] || '';
          } else {
            reason = data;
          }
        }

        const ev = new CloseEvent('close', {
          code,
          reason,
          wasClean: code === 1000,
        });
        this.dispatchEvent(ev);
        if (this.onclose) this.onclose(ev);
        break;
      }

      case 'error': {
        this._readyState = CLOSED;
        _instances.delete(this._connId);

        const errorEv = new Event('error');
        this.dispatchEvent(errorEv);
        if (this.onerror) this.onerror(errorEv);

        // Spec: error is always followed by close (code 1006 = abnormal closure)
        const closeEv = new CloseEvent('close', {
          code: 1006,
          reason: typeof data === 'string' ? data : '',
          wasClean: false,
        });
        this.dispatchEvent(closeEv);
        if (this.onclose) this.onclose(closeEv);
        break;
      }
    }
  }
}
