// @nymproject/mix-fetch
//
// Drop-in `fetch()` replacement that routes HTTP/HTTPS through the Nym mixnet.
// Shares a single mixnet tunnel with @nymproject/mix-dns and
// @nymproject/mix-websocket via @nymproject/mix-tunnel.
//
// v2 surface (clean, no legacy shim):
//
//   import { setupMixTunnel, mixFetch, disconnectMixTunnel } from '@nymproject/mix-fetch';
//   await setupMixTunnel({ preferredIpr, disableCoverTraffic, ... });
//   const res = await mixFetch('https://example.com');
//   await disconnectMixTunnel();
//
// Or, for the "setup + fetch" convenience:
//
//   const mixFetch = await createMixFetch({ ...opts });
//   const res = await mixFetch('https://example.com');
//
// See `SetupMixTunnelOpts` (re-exported from @nymproject/mix-tunnel) for the
// full options surface, which mirrors smolmix-wasm's SetupOpts.

import {
  getMixTunnel,
  setupMixTunnel,
  disconnectMixTunnel,
  getTunnelState,
  SetupMixTunnelOpts,
} from '@nymproject/mix-tunnel';

export { setupMixTunnel, disconnectMixTunnel, getTunnelState };
export type { SetupMixTunnelOpts };

/**
 * Fetch over the mixnet. Drop-in replacement for the browser `fetch()`.
 *
 * Requires the tunnel to be up: call `setupMixTunnel(opts)` first, or use
 * `createMixFetch(opts)` to combine setup + fetch.
 */
export const mixFetch = async (url: string, init?: RequestInit): Promise<Response> => {
  const tunnel = await getMixTunnel();
  // The wasm-side returns a `{body: Uint8Array, status, statusText,
  // headers: [[k,v]...]}` object (see smolmix `serialise_response`).
  // The `Headers` constructor accepts the [[k,v]] pair shape directly so
  // repeated names like `Set-Cookie` survive.
  const raw = await tunnel.mixFetch(url, init ?? {});

  // `raw.body` is `Uint8Array<ArrayBufferLike>`. The ArrayBufferLike side
  // includes SharedArrayBuffer, which the Response constructor's BodyInit
  // doesn't accept. The runtime value is always a non-shared array; cast.
  return new Response(raw.body as BodyInit, {
    status: raw.status,
    statusText: raw.statusText,
    headers: new Headers(raw.headers),
  });
};

/**
 * Convenience: set up the tunnel and return a fetch-bound function. Equivalent
 * to `await setupMixTunnel(opts); return mixFetch;`. Safe to call multiple
 * times; the underlying tunnel is a singleton.
 */
export const createMixFetch = async (opts?: SetupMixTunnelOpts): Promise<typeof mixFetch> => {
  await setupMixTunnel(opts);
  return mixFetch;
};
