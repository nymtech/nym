// Public types for the shared mixnet tunnel.
//
// The tunnel is a single WASM instance with one IPR + smoltcp stack;
// mix-fetch, mix-dns, and mix-websocket all route through it.
//
// The fields below mirror smolmix-wasm's `SetupOpts`. Kept inline (not
// imported) so consumers don't take a transitive type-only dep on
// `@nymproject/smolmix-wasm`, which never publishes to npm.

export interface SetupMixTunnelOpts {
  preferredIpr?: string | undefined;
  preferredGateway?: string | undefined;
  clientId?: string | undefined;
  forceTls?: boolean;
  disablePoissonTraffic?: boolean;
  disableCoverTraffic?: boolean;
  openReplySurbs?: number | undefined;
  dataReplySurbs?: number | undefined;
  primaryDns?: string | undefined;
  fallbackDns?: string | undefined;
  storagePassphrase?: string | undefined;
  connectTimeoutMs?: number | undefined;
  dnsTimeoutMs?: number | undefined;
  tcpKeepaliveMs?: number | undefined;
  tcpBufferSize?: number | undefined;
  maxRedirects?: number | undefined;
  debug?: boolean;
}

export type TunnelStateName = 'connecting' | 'ready' | 'disconnecting' | 'disconnected' | 'failed';

export interface TunnelState {
  state: TunnelStateName;
  reason?: string;
}

/**
 * Pre-serialised response shape produced by `smolmix-wasm::mixFetch`. Designed
 * for Comlink transfer (Uint8Array + primitive arrays survive structured clone).
 *
 * `headers` is a sequence of `[name, value]` pairs rather than a record so that
 * repeated names like `Set-Cookie`, `Vary`, `Link`, `WWW-Authenticate` survive.
 * The TS facade reconstructs a real `Response` via:
 *
 *   new Response(raw.body, {
 *     status: raw.status,
 *     statusText: raw.statusText,
 *     headers: new Headers(raw.headers),
 *   })
 */
export interface MixFetchResponseInit {
  body: Uint8Array;
  status: number;
  statusText: string;
  headers: Array<[string, string]>;
}

export type WsEventType = 'open' | 'text' | 'binary' | 'close' | 'error';
export type WsEventCallback = (handleId: number, type: WsEventType, data: unknown) => void;

export interface IMixTunnelWorker {
  setupMixTunnel(opts?: SetupMixTunnelOpts): Promise<void>;
  disconnectMixTunnel(): Promise<void>;
  getTunnelState(): Promise<TunnelState>;
  mixFetch(url: string, init: unknown): Promise<MixFetchResponseInit>;
  mixDNS(hostname: string): Promise<string>;
  mixWebSocket(url: string, protocols: string[] | undefined, onEvent: WsEventCallback): Promise<number>;
  wsSend(handleId: number, data: string | Uint8Array | ArrayBuffer): Promise<void>;
  wsClose(handleId: number, code: number, reason: string): Promise<void>;
}

export enum EventKinds {
  Loaded = 'Loaded',
}

export interface LoadedEvent {
  kind: EventKinds.Loaded;
  args: { loaded: true };
}
