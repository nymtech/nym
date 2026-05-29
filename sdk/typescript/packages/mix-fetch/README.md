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

The `setupMixTunnel` call accepts the full `SetupMixTunnelOpts` surface — IPR
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

## Migrating from v1.x

The legacy v1.x mix-fetch was a thin wrapper around a Go-based wasm network
stack. v2.x is a thin wrapper around the smolmix-wasm Rust stack. The API
surface is **not** identical; if your v1 code looks like the left column,
update it to look like the right:

| v1.x | v2.x |
|---|---|
| `await createMixFetch({ preferredNetworkRequester, clientId, mixFetchOverride, responseBodyConfigMap })` | `await setupMixTunnel({ preferredIpr, clientId, connectTimeoutMs, ... })` |
| `mixFetch(url, args, opts)` (3-arg) | `mixFetch(url, args)` (2-arg) + `setupMixTunnel(opts)` separately |
| `args.mode = 'unsafe-ignore-cors'` | not needed — the IPR enforces its own egress policy, browser CORS doesn't apply |
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
- **Bundle size**: v2 inlines the wasm + worker into a single ~38 MB ESM
  module. No sibling assets to ship. Trade-off for zero-config deployment.
- **Browser-only**: v2 targets `wasm32-unknown-unknown` and uses a Web Worker
  for the network stack. The v1 `@nymproject/mix-fetch-node` Node entry point
  is not yet ported.

See `@nymproject/mix-tunnel`'s `SetupMixTunnelOpts` for the full v2 options
surface.

## Consumer build requirements

The package ships as raw ESM with a bare `import` of `@nymproject/mix-tunnel`.
Use a bundler that follows package imports (webpack, rollup, parcel, vite,
esbuild). The 38 MB wasm payload lives inside `@nymproject/mix-tunnel`, so
your bundler will surface a single large chunk — plan code-splitting around
it (dynamic `import('@nymproject/mix-fetch')` is the usual move).
