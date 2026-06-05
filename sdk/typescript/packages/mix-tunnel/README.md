# @nymproject/mix-tunnel

Shared mixnet tunnel lifecycle for the smolmix-based SDKs.

This package owns the single Web Worker that hosts `@nymproject/smolmix-wasm`,
so that `@nymproject/mix-fetch`, `@nymproject/mix-dns`, and
`@nymproject/mix-websocket` all share one IPR connection, one smoltcp stack,
and one DNS cache.

Most consumers will not import this directly: import a feature package and
the tunnel comes along.

## Direct usage

```ts
import { setupMixTunnel, disconnectMixTunnel, getTunnelState } from '@nymproject/mix-tunnel';

await setupMixTunnel({ debug: true });

const state = await getTunnelState(); // { state: 'ready' }

// ... use mix-fetch / mix-dns / mix-websocket here ...

await disconnectMixTunnel();
```

## Layout

```
mix-tunnel
  + (owns the Web Worker)
  + (loads smolmix-wasm)
  + exposes Comlink API to:
      |
      +---+--------+----------+
      |            |          |
  mix-fetch    mix-dns    mix-websocket
```

The feature packages call `getMixTunnel()` and invoke the same Comlinked
handle, so the underlying tunnel is shared.
