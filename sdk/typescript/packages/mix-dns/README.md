# @nymproject/mix-dns

Hostname-to-IP resolution over the Nym mixnet. Uses the IP Packet Router's
DNS path (UDP), so no TCP socket or TLS handshake is set up.

## Usage

```ts
import { setupMixTunnel, mixDNS } from '@nymproject/mix-dns';

await setupMixTunnel();

const ip = await mixDNS('example.com');
console.log(ip); // "93.184.216.34"
```

The tunnel is shared with `@nymproject/mix-fetch` and
`@nymproject/mix-websocket` via `@nymproject/mix-tunnel`; calling
`setupMixTunnel` once is enough for all three.

## Consumer build requirements

Ships as raw ESM with a bare `import` of `@nymproject/mix-tunnel`. Use a
bundler that follows package imports (webpack, rollup, parcel, vite,
esbuild). Browser-only target.
