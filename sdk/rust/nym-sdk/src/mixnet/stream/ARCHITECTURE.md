# Stream Multiplexing — Architecture

## Overview

The stream subsystem multiplexes concurrent `AsyncRead + AsyncWrite` byte
channels over a single `MixnetClient`. Each channel is a `MixnetStream`
identified by a random `StreamId`.

```text
┌─────────────────────────────────────────────────────────┐
│                      MixnetClient                       │
│                                                         │
│  ┌──────────────┐   ┌──────────────┐                    │
│  │ MixnetStream │   │ MixnetStream │  ...               │
│  │  (peer A)    │   │  (peer B)    │                    │
│  └──────┬───────┘   └──────┬───────┘                    │
│         │writes            │writes                      │
│         ▼                  ▼                            │
│  ┌─────────────────────────────────┐                    │
│  │     ClientInput.input_sender    │                    │
│  └──────────────┬──────────────────┘                    │
│                 │                                       │
│                 ▼                                       │
│           ── mixnet ──                                  │
│                 │                                       │
│                 ▼                                       │
│  ┌─────────────────────────────────┐                    │
│  │     reconstructed_receiver      │                    │
│  └──────────────┬──────────────────┘                    │
│                 │                                       │
│                 ▼                                       │
│  ┌─────────────────────────────────┐                    │
│  │           Router task           │                    │
│  │  decode header → dispatch by ID │                    │
│  └──┬──────────────────────────┬───┘                    │
│     │ Open messages            │ Data messages          │
│     ▼                          ▼                        │
│  ┌──────────────┐   ┌──────────────────┐                │
│  │MixnetListener│   │ StreamMap lookup │                │
│  │  .accept()   │   │ → per-stream tx  │                │
│  └──────────────┘   └──────────────────┘                │
└─────────────────────────────────────────────────────────┘
```

## Wire Protocol (`protocol.rs`)

Every stream message has a 10-byte header:

```
[Version: 1][StreamId: 8][MessageType: 1][payload...]
```

- `Open` (0) — initiates a new stream
- `Data` (1) — carries payload for an existing stream

There is no `Close` type — the mixnet does not yet guarantee ordering, so
streams clean up via `Drop` and idle timeout instead.

## Initialization

Stream mode activates lazily on the first `open_stream()` or `listener()`
call. This is a one-way transition — message-mode APIs
(`send_plain_message`, `wait_for_messages`, etc.) return
`Error::StreamModeActive` afterwards.

On activation, `reconstructed_receiver` is handed to the router task
exclusively.

## Router Task (`run_router`)

A background task that reads inbound messages and dispatches them:

- **Open** → forwarded to `MixnetListener`'s accept channel
- **Data** → looked up in `StreamMap` by `StreamId`, forwarded to the
  stream's channel
- Unrecognised messages are silently dropped

Shuts down via `CancellationToken` or when the receiver closes.

## Stream Lifecycle

**Outbound** (`open_stream`): generates a random `StreamId`, registers in
`StreamMap`, sends an `Open` message, returns a `MixnetStream`.

**Inbound** (`MixnetListener::accept`): receives an `InboundOpen` from the
router, registers in `StreamMap`, returns a `MixnetStream` using the
sender's reply SURBs.

## Cleanup

- **`Drop` on `MixnetStream`** — deregisters from `StreamMap`
- **`poll_shutdown`** — same, with a `deregistered` flag to avoid double-remove
- **Idle timeout** — streams inactive longer than `stream_idle_timeout`
  (default 30 min) are reaped every 10s

## `StreamMap`

`Arc<Mutex<HashMap<StreamId, StreamEntry>>>` — shared between router,
streams, and listener. Methods: `register_stream`, `remove`,
`send_to_stream`, `cleanup_stale`.

## Known Limitations

- **No message ordering** — large writes may arrive out of order; no
  `Close` message. Ordering is planned for a future release.
- **No protocol discriminator** — stream and plain messages are
  indistinguishable on the wire. Planned for a future release.
