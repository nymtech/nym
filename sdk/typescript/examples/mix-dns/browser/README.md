# mixDNS Usage Example

Shows how to resolve hostnames over the Nym mixnet using
`@nymproject/mix-dns`. Resolution travels the IPR's DNS path (UDP, no TCP/TLS).

```ts
import { setupMixTunnel, mixDNS } from '@nymproject/mix-dns';

await setupMixTunnel();
const ip = await mixDNS('nymtech.net');
```

## Running the example

```
npm install
npm run start
```

Open http://localhost:1234. The example resolves three hostnames and prints
the round-trip time for each.

## Sharing the tunnel

`setupMixTunnel()` is a no-op after the first call across all three smolmix
SDKs (`mix-fetch`, `mix-dns`, `mix-websocket`). If you already imported one
of the others, you can skip the setup line.
