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
import type { SetupOpts as WasmSetupOpts } from '@nymproject/smolmix-wasm';
import * as Comlink from 'comlink';
import { EventKinds, IMixTunnelWorker, LoadedEvent, SetupMixTunnelOpts } from '../types';

// Compile-time guard: the hand-written SetupMixTunnelOpts (../types) must mirror
// smolmix-wasm's generated SetupOpts exactly, minus the TS-only `debug` toggle
// this worker strips before the wasm call. If the Rust SetupOpts gains, drops,
// or retypes a field and ../types does not follow, _AssertOptsMatch stops being
// `true` and the build fails. Type-level only, so nothing is emitted, and the
// type-only import keeps smolmix-wasm out of the published type surface.
type _AssertEq<A, B> = [keyof A] extends [keyof B]
  ? [keyof B] extends [keyof A]
    ? A extends B
      ? B extends A
        ? true
        : false
      : false
    : false
  : false;
type _AssertTrue<T extends true> = T;
type _AssertOptsMatch = _AssertTrue<_AssertEq<Omit<SetupMixTunnelOpts, 'debug'>, WasmSetupOpts>>;

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
