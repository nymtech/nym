# @nymproject/mix-fetch

Drop-in replacement for `fetch()` that routes HTTP/HTTPS through the Nym
mixnet.

## Usage

```ts
import { setupMixTunnel, mixFetch } from '@nymproject/mix-fetch';

await setupMixTunnel();

const res = await mixFetch('https://example.com/');
const html = await res.text();
```

The `setupMixTunnel` call accepts the full `SetupMixTunnelOpts` surface: IPR
pinning, cover-traffic toggles, SURB budgets, DNS overrides, TCP/connect
timeouts, etc. See `@nymproject/mix-tunnel`'s typings for the complete list.

### Convenience: setup + fetch in one call

```ts
import { createMixFetch } from '@nymproject/mix-fetch';

const mixFetch = await createMixFetch({ preferredIpr: '...' });
const res = await mixFetch('https://example.com/');
```

### Shared tunnel

Setting up the tunnel once unlocks all three smolmix SDKs simultaneously:

```ts
import { setupMixTunnel, mixFetch } from '@nymproject/mix-fetch';
import { mixDNS } from '@nymproject/mix-dns';
import { MixWebSocket } from '@nymproject/mix-websocket';

await setupMixTunnel();

await mixFetch('https://example.com/');
await mixDNS('example.com');
const ws = new MixWebSocket('wss://echo.websocket.events');
```

All three packages delegate to `@nymproject/mix-tunnel`, which owns the single
Web Worker hosting `@nymproject/smolmix-wasm`.

## Default request headers

`mixFetch` ships a small browser-shape header shim. If the caller doesn't set
these headers, smolmix-wasm fills them in before the request leaves the
tunnel. Caller-supplied values always win.

| Header | Injected default |
|--------|------------------|
| `User-Agent` | `Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36` |
| `Accept` | `text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8` |
| `Accept-Language` | `en-US,en;q=0.9` |
| `Accept-Encoding` | `identity` (the wasm build has no decompressor) |

Rationale: many CDNs (cloudflare bot management) and host policies (wikimedia)
reject requests that lack browser-canonical headers. The shim is a floor:
it does not attempt TLS-fingerprint or HTTP/2 impersonation, just the
header-shaped tells. See the smolmix-wasm README "Browser-shape header
shim" section for the full story and the JA3 caveats.

Override per-request like any other header:

```ts
const res = await mixFetch('https://example.com', {
  headers: { 'User-Agent': 'my-app/1.0' },
});
```

## Migrating from v1.x

The legacy v1.x mix-fetch was a thin wrapper around a Go-based wasm network
stack. v2.x is a thin wrapper around the smolmix-wasm Rust stack. The API
surface is **not** identical; if your v1 code looks like the left column,
update it to look like the right:

| v1.x | v2.x |
|---|---|
| `await createMixFetch({ preferredNetworkRequester, clientId, mixFetchOverride, responseBodyConfigMap })` | `await setupMixTunnel({ preferredIpr, clientId, connectTimeoutMs, ... })` |
| `mixFetch(url, args, opts)` (3-arg) | `mixFetch(url, args)` (2-arg) + `setupMixTunnel(opts)` separately |
| `args.mode = 'unsafe-ignore-cors'` | not needed; the IPR enforces its own egress policy, browser CORS doesn't apply |
| `disconnectMixFetch()` | `disconnectMixTunnel()` |

Notable differences:

- **Gateway routing**: v1's `preferredGateway` and `preferredNetworkRequester`
  are gone. v2 uses smolmix's IPR auto-discovery by default; pin one with
  `preferredIpr` if needed.
- **Response body handling**: v1's `responseBodyConfigMap` (used to opt
  particular MIME types into specific body parsers) is gone. v2 returns a
  real `Response` object; call `.text()`, `.arrayBuffer()`, `.json()`,
  `.blob()` as usual.
- **Cover traffic**: v1's `clientOverride.coverTraffic` is now flat opts
  (`disableCoverTraffic`, `disablePoissonTraffic`).
- **Bundle size**: v2 inlines the wasm + worker into a single ESM module.
  No sibling assets to ship, at the cost of a large single chunk. Plan
  code-splitting around it (dynamic `import('@nymproject/mix-fetch')` is
  the usual move).
- **Runtime target**: v2 ships a single ESM bundle that runs in any environment
  exposing `Worker`, `WebAssembly`, `Blob`, and `URL.createObjectURL`. That
  covers modern browsers, Electron renderers, and mobile WebViews (Capacitor,
  Cordova, Ionic, iOS WKWebView, Android WebView). The v1
  `@nymproject/mix-fetch-node` companion for Node is not yet ported to the
  smolmix backend.

See `@nymproject/mix-tunnel`'s `SetupMixTunnelOpts` for the full v2 options
surface.

## Consumer build requirements

Ships as raw ESM with a bare `import` of `@nymproject/mix-tunnel`. Use a
bundler that follows package imports (webpack, rollup, parcel, vite,
esbuild).

Runs in any environment exposing `Worker`, `WebAssembly`, `Blob`, and
`URL.createObjectURL`. That covers modern browsers, Electron renderers,
and mobile WebViews (Capacitor, Cordova, Ionic, iOS WKWebView, Android
WebView). A Node-direct entry point is not yet ported from v1.

The wasm payload lives inside `@nymproject/mix-tunnel`, so your bundler
will surface a single large chunk. Plan code-splitting around it
(dynamic `import('@nymproject/mix-fetch')` is the usual move).
