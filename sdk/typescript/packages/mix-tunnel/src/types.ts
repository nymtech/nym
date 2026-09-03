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

export type TaskName = 'bridge' | 'reactor';

export type FailureReason = { kind: 'task_exited'; task: TaskName } | { kind: 'task_panicked' };

// The names and shape below are the serde output of `TunnelState` in
// wasm/smolmix/src/state.rs, where tsify generates the authoritative type into the
// wasm `.d.ts`. Kept inline here to avoid a dependency on the unpublished
// @nymproject/smolmix-wasm; keep it in step with that generated type.
export type TunnelStateName = 'connecting' | 'ready' | 'shutting_down' | 'shutdown' | 'failed';

// The variants are named rather than written inline so typedoc renders them as
// links; a union of anonymous object literals renders as `object | object | ...`.
// Naming them also lets consumers write narrowing helpers against a variant, e.g.
// `(s: TunnelState): s is TunnelFailed`.
export interface TunnelConnecting {
  state: 'connecting';
}

export interface TunnelReady {
  state: 'ready';
}

export interface TunnelShuttingDown {
  state: 'shutting_down';
}

export interface TunnelShutdown {
  state: 'shutdown';
}

export interface TunnelFailed {
  state: 'failed';
  reason: FailureReason;
}

export type TunnelState =
  | TunnelConnecting
  | TunnelReady
  | TunnelShuttingDown
  | TunnelShutdown
  | TunnelFailed;

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
