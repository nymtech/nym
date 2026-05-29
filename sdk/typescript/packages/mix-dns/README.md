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
