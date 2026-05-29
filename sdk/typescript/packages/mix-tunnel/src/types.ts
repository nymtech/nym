// Public types for the shared mixnet tunnel.
//
// The tunnel is a single WASM instance with one IPR + smoltcp stack;
// mix-fetch, mix-dns, and mix-websocket all route through it.

import type { SetupOpts as SmolmixSetupOpts } from '@nymproject/smolmix-wasm';

/**
 * Options passed to `setupMixTunnel`. Mirrors the full smolmix-wasm `SetupOpts`
 * surface so consumers can reach every tuning knob the wasm exposes — IPR
 * pinning (`preferredIpr`), cover-traffic + Poisson toggles, SURB budgets,
 * DNS server overrides, TCP/connect timeouts, etc.
 *
 * One extra knob lives at the TS layer: `debug`. The wasm has it as a separate
 * runtime call (`setDebugLogging(bool)`); we collapse it into the opts bag so
 * callers can set everything in one place.
 */
export interface SetupMixTunnelOpts extends SmolmixSetupOpts {
  /** Toggle smolmix's verbose console tracing. Routed to
   * `smolmix-wasm::setDebugLogging` at setup time. Off by default. */
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
