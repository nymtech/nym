# Client Pool — Architecture

## Overview

`ClientPool` maintains a configurable number of pre-connected ephemeral
`MixnetClient` instances, eliminating per-request connection latency
(gateway handshake, key generation, topology fetch).

```text
┌──────────────────────────────────────────────────────┐
│                     ClientPool                        │
│                                                      │
│   ┌────────────┐ ┌────────────┐ ┌────────────┐      │
│   │  Client 1  │ │  Client 2  │ │  Client 3  │ ...  │
│   │ (connected)│ │ (connected)│ │ (connected)│      │
│   └────────────┘ └────────────┘ └────────────┘      │
│                                                      │
│   start() loop:                                      │
│     if len < reserve → create new client              │
│     if len >= reserve → sleep                         │
│     if cancel_token → break                          │
└──────────────────────────────────────────────────────┘
         │
         │ get_mixnet_client()
         ▼
    Arc::try_unwrap(client) → MixnetClient
```

## Lifecycle

1. **Create** — `ClientPool::new(reserve)` creates an empty pool.
2. **Start** — `pool.start().await` runs a background loop that keeps
   the pool topped up to `reserve` connected clients.
3. **Get** — `pool.get_mixnet_client().await` pops a client (returns
   `None` if empty). Clients are consumed, not returned — the background
   loop creates replacements.
4. **Shutdown** — `pool.disconnect_pool().await` cancels the loop and
   disconnects all remaining clients.

## Integration with TcpProxy

`NymProxyClient` uses a `ClientPool` internally. Each TCP connection
pops a client; if the pool is empty, an ephemeral client is created
on the fly. Set reserve to 0 to always create on-demand.
