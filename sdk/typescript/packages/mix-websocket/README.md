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
