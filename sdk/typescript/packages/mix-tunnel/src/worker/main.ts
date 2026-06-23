/* eslint-disable no-restricted-globals */
import {
  setupMixTunnel,
  disconnectMixTunnel,
  getTunnelState,
  mixFetch,
  mixDNS,
  mixWebSocket,
  wsSend,
  wsClose,
  setDebugLogging,
} from '@nymproject/smolmix-wasm';
import * as Comlink from 'comlink';
import { EventKinds, IMixTunnelWorker, LoadedEvent } from '../types';

const postMessageWithType = <E>(event: E) => self.postMessage(event);

export async function run() {
  const api: IMixTunnelWorker = {
    setupMixTunnel: async (opts) => {
      // `debug` is a TS-only convenience; smolmix exposes it as a separate
      // runtime toggle. Apply before setup so start-up logs appear.
      const { debug, ...wasmOpts } = opts ?? {};
      if (debug !== undefined) setDebugLogging(debug);
      await setupMixTunnel(wasmOpts);
    },
    disconnectMixTunnel: () => disconnectMixTunnel(),
    getTunnelState: async () => getTunnelState(),
    mixFetch: (url, init) => mixFetch(url, init),
    mixDNS: (hostname) => mixDNS(hostname),
    mixWebSocket: (url, protocols, onEvent) => mixWebSocket(url, protocols, onEvent),
    wsSend: async (handleId, data) => wsSend(handleId, data),
    wsClose: async (handleId, code, reason) => wsClose(handleId, code, reason),
  };

  Comlink.expose(api);
  postMessageWithType<LoadedEvent>({ kind: EventKinds.Loaded, args: { loaded: true } });
}
