# mixWebSocket Usage Example

Shows how to open a WSS connection over the Nym mixnet using
`@nymproject/mix-websocket`. The `MixWebSocket` class mirrors the browser
`WebSocket` API where it makes sense.

```ts
import { setupMixTunnel, MixWebSocket } from '@nymproject/mix-websocket';

await setupMixTunnel();
const ws = new MixWebSocket('wss://echo.websocket.events');
await ws.opened();
ws.addEventListener('message', (e) => console.log(e.data));
await ws.send('hello');
```

## Running the example

```
npm install
npm run start
```

Open http://localhost:1234. The example echoes whatever you send via
`echo.websocket.events`.

## Differences from the browser WebSocket

- `await ws.opened()` blocks until the upgrade completes.
- `binaryType` is fixed to `arraybuffer`.
- No `bufferedAmount`; writes queue through the tunnel worker.

## Sharing the tunnel

`setupMixTunnel()` is shared across `mix-fetch`, `mix-dns`, and
`mix-websocket`. If another of those is already initialised, you can skip
the setup line.
