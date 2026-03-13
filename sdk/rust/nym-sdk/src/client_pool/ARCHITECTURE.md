# Client Pool — Architecture

## Overview

The `ClientPool` maintains a configurable number of connected ephemeral
`MixnetClient` instances, ready for immediate use. This eliminates the
latency of connecting a new client on each request — gateway handshake,
key generation, and topology fetch are done ahead of time.

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
│     if len >= reserve → sleep 100ms                   │
│     if cancel_token → break                          │
└──────────────────────────────────────────────────────┘
         │
         │ get_mixnet_client()
         ▼
    Arc::try_unwrap(client) → MixnetClient
         │
         │ (use it, then disconnect)
         ▼
       dropped
```

## Lifecycle

### Creation

```rust
let pool = ClientPool::new(5);  // maintain 5 clients in reserve
```

The pool starts empty. No clients are created until `start()` is called.

### Background Loop

`pool.start().await` runs a loop that:

1. Checks current pool size against the reserve target
2. If below target: creates a new ephemeral `MixnetClient` via
   `MixnetClientBuilder::new_ephemeral()`, retrying on failure
3. If at target: sleeps for 100ms
4. Repeats until the `CancellationToken` fires

Each client connects to a random gateway (or a specified one via
`start_with_specified_gateway()`).

### Getting a Client

```rust
if let Some(client) = pool.get_mixnet_client().await {
    // client is removed from pool — pool will create a replacement
    // use client...
    client.disconnect().await;
}
```

Clients are **consumed, not returned**. Once taken from the pool,
the background loop notices the shortfall and creates a replacement.

If the pool is empty, `get_mixnet_client()` returns `None` — the
caller must decide whether to wait, create an ephemeral client
directly, or fail.

### Shutdown

```rust
pool.disconnect_pool().await;
```

This cancels the background loop and disconnects all remaining
clients in the pool. The pool cannot be restarted after shutdown.

## Shared State

The pool's client list is `Arc<RwLock<Vec<Arc<MixnetClient>>>>`:

- `start()` takes a **write lock** to push new clients
- `get_mixnet_client()` takes a **write lock** to pop a client
- `get_client_count()` takes a **read lock** for inspection
- The `ClientPool` struct itself is `Clone` (shared `Arc` internals)

## Integration with TcpProxy

The `NymProxyClient` uses a `ClientPool` internally. Each incoming TCP
connection pops a client from the pool. If the pool is empty, the proxy
creates an ephemeral client on the fly (with higher latency). This is
configurable — set the reserve to 0 to always create on-demand.
