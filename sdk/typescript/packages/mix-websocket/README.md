# @nymproject/mix-websocket

WebSocket-like class for sending WS/WSS traffic through the Nym mixnet.

## Usage

```ts
import { setupMixTunnel, MixWebSocket } from '@nymproject/mix-websocket';

await setupMixTunnel();

const ws = new MixWebSocket('wss://echo.websocket.events');
await ws.opened();

ws.addEventListener('message', (event) => {
  console.log('received', event.data);
});

await ws.send('hello');
await ws.close(1000, 'done');
```

## Differences from the browser WebSocket

- The constructor returns immediately, but the upgrade happens asynchronously.
  Use `await ws.opened()` if you need to block until the handshake completes.
- `binaryType` is fixed to `arraybuffer`; there is no Blob mode.
- There is no `bufferedAmount`; writes queue through the tunnel worker.

The tunnel is shared with `@nymproject/mix-fetch` and `@nymproject/mix-dns`
via `@nymproject/mix-tunnel`; calling `setupMixTunnel` once is enough.

## Consumer build requirements

Ships as raw ESM with a bare `import` of `@nymproject/mix-tunnel`. Use a
bundler that follows package imports (webpack, rollup, parcel, vite,
esbuild).

Runs in any environment exposing `Worker`, `WebAssembly`, `Blob`, and
`URL.createObjectURL`. That covers modern browsers, Electron renderers,
and mobile WebViews (Capacitor, Cordova, Ionic, iOS WKWebView, Android
WebView). A Node-direct entry point is not yet ported from v1.
