// Public entry point for @nymproject/mix-tunnel.
//
// Owns the single Web Worker that hosts smolmix-wasm. Feature packages
// (@nymproject/mix-fetch, @nymproject/mix-dns, @nymproject/mix-websocket)
// import `getMixTunnel()` and call into the same worker, so they share
// one IPR connection + one smoltcp stack + one DNS cache.

import InlineWasmWebWorker from 'web-worker:./worker/worker';
import * as Comlink from 'comlink';
import { EventKinds, IMixTunnelWorker } from './types';

export * from './types';

let tunnelPromise: Promise<Comlink.Remote<IMixTunnelWorker>> | undefined;

const createWorker = async (): Promise<Worker> =>
  new Promise<Worker>((resolve, reject) => {
    const worker = new InlineWasmWebWorker();
    worker.addEventListener('error', reject);
    worker.addEventListener(
      'message',
      (msg: MessageEvent) => {
        worker.removeEventListener('error', reject);
        if (msg.data?.kind === EventKinds.Loaded) {
          resolve(worker);
        } else {
          reject(msg);
        }
      },
      { once: true },
    );
  });

const createTunnel = async (): Promise<Comlink.Remote<IMixTunnelWorker>> => {
  const worker = await createWorker();
  return Comlink.wrap<IMixTunnelWorker>(worker);
};

/**
 * Get the singleton tunnel worker handle. The first call spawns the worker
 * and loads smolmix-wasm; subsequent calls return the same handle.
 *
 * Note: this does NOT call `setupMixTunnel` automatically. Call it on the
 * returned handle (or use the top-level `setupMixTunnel` helper) before
 * issuing fetch/dns/websocket requests.
 */
export const getMixTunnel = async (): Promise<Comlink.Remote<IMixTunnelWorker>> => {
  if (!tunnelPromise) {
    tunnelPromise = createTunnel();
  }
  return tunnelPromise;
};

/** Initialise the mixnet tunnel. Idempotent; safe to call from multiple feature packages. */
export const setupMixTunnel = async (opts?: import('./types').SetupMixTunnelOpts): Promise<void> => {
  const tunnel = await getMixTunnel();
  await tunnel.setupMixTunnel(opts);
};

/** Tear the tunnel down. After this, the WASM is unusable until page reload. */
export const disconnectMixTunnel = async (): Promise<void> => {
  if (!tunnelPromise) return;
  const tunnel = await tunnelPromise;
  await tunnel.disconnectMixTunnel();
};

/** Inspect the current tunnel state. Pre-setup reads as `connecting`. */
export const getTunnelState = async (): Promise<import('./types').TunnelState> => {
  const tunnel = await getMixTunnel();
  return tunnel.getTunnelState();
};

/**
 * Re-export of `Comlink.proxy` so feature packages (mix-websocket etc.) can
 * mark callbacks for proxy-transfer using THIS module's Comlink instance.
 *
 * Why: Comlink detects proxy-marked values via a `Symbol('Comlink.proxy')`.
 * That symbol is created per module instance, so if mix-websocket bundled its
 * own Comlink, `mix-websocket.Comlink.proxy(fn)` would mark `fn` with a
 * symbol that mix-tunnel's serializer doesn't recognise, falling through to
 * structured-clone, which can't clone functions, throws DOMException.
 *
 * By exposing this re-export, mix-websocket's `import { proxy } from
 * '@nymproject/mix-tunnel'` returns the same function backed by the same
 * Comlink instance, so the marker symbol matches.
 */
export { proxy } from 'comlink';
