# Mixnet Module — Architecture

## Overview

The `mixnet` module provides `MixnetClient` — the main handle for
connecting to the Nym mixnet, sending messages through Sphinx-encrypted
multi-hop routing, and receiving reconstructed messages.

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

1. **Build** — `MixnetClientBuilder` configures endpoints, storage,
   gateway preference, and optional stream settings.
   Shorthand: `MixnetClient::connect_new().await`.
2. **Connect** — `.build()?.connect_to_mixnet().await?` yields a
   connected `MixnetClient`.
3. **Use** — send/receive in one of two modes (see below).
4. **Disconnect** — `client.disconnect().await` shuts down all
   background tasks.

## Two Operating Modes

**Message mode** (default): raw payload send/receive.
- `send_plain_message`, `send_message`, `send_reply`
- `wait_for_messages` / `Stream<Item = ReconstructedMessage>`

**Stream mode**: persistent `AsyncRead + AsyncWrite` channels.
- `open_stream(recipient, reply_surbs)` → `MixnetStream`
- `client.listener()` → `MixnetListener` → `.accept()`
- One-way transition — message-mode methods return
  `Error::StreamModeActive` once activated.
- See the [`stream`] submodule for details.

## Key Types

| Type | Role |
|---|---|
| `MixnetClient` | Connected client handle |
| `MixnetClientSender` | Clone-able send-only handle (`split_sender()`) |
| `MixnetClientBuilder` | Configures and connects a client |
| `DisconnectedMixnetClient` | After `build()`, before `connect_to_mixnet()` |
| `MixnetMessageSender` | Trait shared by `MixnetClient` and `MixnetClientSender` |
| `MixnetStream` | Single `AsyncRead + AsyncWrite` byte channel |
| `MixnetListener` | Accepts inbound streams |
| `Recipient` | Nym address (`identity.encryption@gateway`) |

## Storage

- **`Ephemeral`** — in-memory, keys discarded on disconnect
- **`OnDiskPersistent`** — keys and gateway registration persisted to disk

## Sub-modules

| Module | Purpose |
|---|---|
| `client` | `MixnetClientBuilder`, `DisconnectedMixnetClient` |
| `native_client` | `MixnetClient`, `MixnetClientSender` |
| `stream` | Stream multiplexing (`MixnetStream`, `MixnetListener`) |
| `traits` | `MixnetMessageSender` trait |
| `socks5_client` | SOCKS5 proxy client variant |
