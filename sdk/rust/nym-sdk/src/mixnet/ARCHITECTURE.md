# Mixnet Module — Architecture

## Overview

The `mixnet` module is the core of the Nym SDK. It provides `MixnetClient`,
which handles connecting to the Nym mixnet, sending messages through Sphinx
packet encryption and 5-hop routing, and receiving reconstructed messages
on the other side.

```text
                      User code
                         │
           ┌─────────────┴─────────────┐
           │       MixnetClient         │
           │                            │
  send─────┤  client_input ──────────►  │──► Sphinx packets ──► mixnet
           │                            │
  recv◄────┤  reconstructed_receiver ◄──│◄── reconstructed ◄── mixnet
           │                            │
           │  client_state (topology,   │
           │   queue lengths, etc.)     │
           └────────────────────────────┘
```

## Client Lifecycle

### Building

`MixnetClientBuilder` configures the client before connection:

```text
MixnetClientBuilder::new_ephemeral()     ← in-memory keys, no persistence
MixnetClientBuilder::new_with_default_storage(path)  ← on-disk keys
    │
    ├─ .network_details(...)             ← API endpoints, contract addresses
    ├─ .custom_topology_provider(...)    ← override topology source
    ├─ .request_gateway("identity")      ← pick a specific gateway
    ├─ .with_stream_idle_timeout(dur)    ← configure stream cleanup
    ├─ .debug_config(...)                ← tuning knobs
    │
    ▼
    .build()?                            → DisconnectedMixnetClient
    │
    ▼
    .connect_to_mixnet().await?          → MixnetClient (connected)
```

The shorthand `MixnetClient::connect_new().await` does all of the above
with ephemeral storage and default settings.

### Connected State

Once connected, a `MixnetClient` owns:

| Field | Purpose |
|---|---|
| `client_input` | Channel to inject outbound messages |
| `reconstructed_receiver` | Channel delivering inbound messages after Sphinx reconstruction |
| `client_state` | Topology, queue lengths, connection info |
| `shutdown_handle` | Manages all background tasks |
| `nym_address` | This client's `Recipient` address (`identity.encryption@gateway`) |

### Two Operating Modes

The client operates in one of two mutually exclusive modes:

**Message mode** (default) — send and receive raw message payloads:
- `send_plain_message(recipient, data)` — send with default SURBs
- `send_message(recipient, data, surbs)` — send with explicit SURB count
- `send_reply(sender_tag, data)` — reply via SURBs
- `wait_for_messages()` — poll for incoming messages
- Also implements `futures::Stream<Item = ReconstructedMessage>`

**Stream mode** — persistent `AsyncRead + AsyncWrite` channels:
- `open_stream(recipient, reply_surbs)` — open a stream to a peer
- `listener()` — accept inbound streams
- See the [`stream`] submodule for details

Stream mode is activated on the first call to `open_stream()` or
`listener()`. Once active, message-mode methods return
`Error::StreamModeActive`. This is a one-way transition — there is no
way to switch back to message mode without disconnecting.

### Disconnecting

`client.disconnect().await` shuts down all background tasks (gateway
connection, cover traffic, topology refresh) and drops the client.

## Key Types

- **`MixnetClient`** — the connected client handle
- **`MixnetClientSender`** — a clone-able send-only handle (via `client.split_sender()`)
- **`MixnetClientBuilder`** — configures and connects a client
- **`DisconnectedMixnetClient`** — intermediate state after `build()`, before `connect_to_mixnet()`
- **`MixnetMessageSender`** — trait shared by `MixnetClient` and `MixnetClientSender`
- **`Recipient`** — a Nym network address (`identity.encryption@gateway`)
- **`ReconstructedMessage`** — an inbound message after Sphinx decryption

## Storage

Two storage backends:

- **`Ephemeral`** — in-memory, keys discarded on disconnect
- **`OnDiskPersistent`** — keys and gateway registration saved to disk,
  survives restarts with the same identity

## Sub-modules

| Module | Purpose |
|---|---|
| `client` | `MixnetClientBuilder` and `DisconnectedMixnetClient` |
| `native_client` | `MixnetClient` and `MixnetClientSender` implementation |
| `stream` | Stream multiplexing (`MixnetStream`, `MixnetListener`) |
| `traits` | `MixnetMessageSender` trait |
| `config` | Client configuration |
| `sink` | `MixnetMessageSink` for use with tokio codec pipelines |
| `socks5_client` | SOCKS5 proxy client variant |
| `paths` | Storage path helpers |
