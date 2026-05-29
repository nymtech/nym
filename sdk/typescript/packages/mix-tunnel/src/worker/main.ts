/* eslint-disable no-restricted-globals */
import {
  setupMixTunnel as wasmSetupMixTunnel,
  disconnectMixTunnel as wasmDisconnectMixTunnel,
  getTunnelState as wasmGetTunnelState,
  mixFetch as wasmMixFetch,
  mixDNS as wasmMixDNS,
  mixWebSocket as wasmMixWebSocket,
  wsSend as wasmWsSend,
  wsClose as wasmWsClose,
  setDebugLogging as wasmSetDebugLogging,
} from '@nymproject/smolmix-wasm';
import * as Comlink from 'comlink';
import { EventKinds, IMixTunnelWorker, LoadedEvent, MixFetchResponseInit } from '../types';

// `self` in a DedicatedWorkerGlobalScope has its own postMessage that does
// not take a target origin (unlike Window.postMessage).
const postMessageWithType = <E>(event: E) =>
  (self as unknown as DedicatedWorkerGlobalScope).postMessage(event);

export async function run() {
  const api: IMixTunnelWorker = {
    setupMixTunnel: async (opts) => {
      // `debug` is a TS-only convenience; the wasm has a separate runtime
      // toggle. Apply it before setup so smolmix's start-up logs appear.
      const tunnelOpts = { ...(opts ?? {}) } as Record<string, unknown>;
      const debug = tunnelOpts.debug as boolean | undefined;
      delete tunnelOpts.debug;
      if (debug !== undefined) {
        wasmSetDebugLogging(debug);
      }
      await wasmSetupMixTunnel(tunnelOpts as any);
    },
    disconnectMixTunnel: async () => {
      await wasmDisconnectMixTunnel();
    },
    getTunnelState: async () => wasmGetTunnelState() as any,
    mixFetch: async (url, init) => {
      // smolmix-wasm already returns a plain `{body: Uint8Array, status,
      // statusText, headers: [[k,v]...]}` object shaped for Comlink transfer
      // (see smolmix `serialise_response`). Pass it through unchanged; the
      // feature package wraps it in a real Response.
      const raw = await wasmMixFetch(url, init);
      return raw as unknown as MixFetchResponseInit;
    },
    mixDNS: async (hostname) => wasmMixDNS(hostname) as unknown as Promise<string>,
    mixWebSocket: async (url, protocols, onEvent) => {
      try {
        const handle = await wasmMixWebSocket(url, protocols as any, onEvent as any);
        return handle as unknown as number;
      } catch (e) {
        // Surface enough detail that the main-thread error handler can log
        // the actual cause rather than the bare `error` event.
        // eslint-disable-next-line no-console
        console.error('[mix-tunnel worker] mixWebSocket failed:', e);
        throw e;
      }
    },
    wsSend: async (handleId, data) => {
      wasmWsSend(handleId, data);
    },
    wsClose: async (handleId, code, reason) => {
      wasmWsClose(handleId, code, reason);
    },
  };

  Comlink.expose(api);
  postMessageWithType<LoadedEvent>({ kind: EventKinds.Loaded, args: { loaded: true } });
}
