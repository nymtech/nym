# @nymproject/mix-dns

Hostname-to-IP resolution over the Nym mixnet. Resolves over DNS-over-HTTPS
(DoH), so each lookup is an HTTPS request (TCP plus TLS) to a DoH resolver,
made from the mixnet exit rather than from the client.

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
esbuild).

Runs in any environment exposing `Worker`, `WebAssembly`, `Blob`, and
`URL.createObjectURL`. That covers modern browsers, Electron renderers,
and mobile WebViews (Capacitor, Cordova, Ionic, iOS WKWebView, Android
WebView). A Node-direct entry point is not yet ported from v1.
