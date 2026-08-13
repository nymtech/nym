---
title: Get started with mix-tunnel
description: Install @nymproject/mix-tunnel and bring up the shared mixnet tunnel.
url: https://nym.com/docs/developers/mix-tunnel/get-started
---

# Get started

## Installation

```bash
npm install @nymproject/mix-tunnel
```

The package ships ESM only. The bundled Web Worker and smolmix-wasm are inlined, so no separate WASM-loading config is needed in Webpack, Vite, or esbuild.

The smolmix-family packages (`mix-tunnel`, `mix-fetch`, `mix-dns`, `mix-websocket`) are new and version together. The API is still settling: pin exact versions and expect breaking changes between releases until the family reaches a stable line. `mix-fetch` already went through a [v1 to v2 clean break](/developers/mix-fetch/migration).

## Quick start

```ts
import { setupMixTunnel, getTunnelState, disconnectMixTunnel } from '@nymproject/mix-tunnel';

// Brings the tunnel up. Call once per page; a second call rejects.
await setupMixTunnel();

// { state: 'ready' } once the IPR handshake completes.
console.log(await getTunnelState());

// Tear down before page unload. The WASM is unusable after this until reload.
window.addEventListener('beforeunload', () => { disconnectMixTunnel(); });
```

After `setupMixTunnel()` resolves, [`mixFetch()`](/developers/mix-fetch), [`mixDNS()`](/developers/mix-dns), and `new MixWebSocket()` (from [`mix-websocket`](/developers/mix-websocket)) all become usable.

Bring the tunnel up live in the [mixnet playground](/developers/playground) and inspect its state over the live mixnet.
