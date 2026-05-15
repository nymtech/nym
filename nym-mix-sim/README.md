# nym-mix-sim

A discrete-time simulator for the Nym mixnet, intended for local testing and experimentation. It models a small network of mix nodes and clients exchanging UDP packets on localhost, allowing you to observe packet flow, experiment with different drivers, and debug routing behaviour step by step.

## Overview

The simulator runs a configurable number of mix nodes and clients on localhost, each bound to its own UDP port. Time advances in **ticks** — each tick runs the client phase, then drains incoming sockets, processes packets through the mixing pipeline, and dispatches outgoing packets.

Two binaries are provided:

| Binary | Purpose |
|--------|---------|
| `nym-mix-sim` | Main simulator: topology generation and tick-loop execution |
| `mix-client` | Standalone tool to inject messages into a running simulation |

## Quick Start

```bash
# 1. Generate a topology with 6 nodes and 2 clients
cargo run --bin nym-mix-sim -- init-topology

# 2. Run the simulation (automatic mode, 1ms ticks, default discrete-sphinx driver)
cargo run --bin nym-mix-sim -- run

# 3. In a separate terminal, send a message between the two clients
cargo run --bin mix-client -- --src 6 --dst 7
# Then type a message and press ENTER
```

## Commands

### `init-topology`

Generates a `topology.json` file describing nodes and clients.

```
cargo run --bin nym-mix-sim -- init-topology [OPTIONS]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--nodes <N>` | `6` | Number of mix nodes |
| `--clients <N>` | `2` | Number of clients |
| `--output <PATH>` | `topology.json` | Output file path |

Nodes are assigned sequential ports starting at `127.0.0.1:9000`. Clients get two sockets each: a mix-facing socket starting at `127.0.0.1:9500` and an app-facing socket starting at `127.0.0.1:9600`. Each node gets a freshly generated X25519 key pair (used by Sphinx drivers).

### `run`

Starts the simulation loop.

```
cargo run --bin nym-mix-sim -- run [OPTIONS]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--topology <PATH>` | `topology.json` | Topology file to load |
| `--driver <DRIVER>` | `discrete-sphinx` | Simulation driver (see below) |
| `--tick-duration-ms <MS>` | `1` | Milliseconds between automatic ticks |
| `--manual` | off | Enable manual stepping mode (ENTER per tick) |
| `--no-display-state` | off | Suppress per-phase state dump in manual mode |

### `mix-client`

Injects messages into a running simulation from stdin.

```
cargo run --bin mix-client -- --src <ID> --dst <ID> [--topology <PATH>]
```

Reads lines from stdin and sends each as a payload routed from client `--src` through the mix network to client `--dst`. Client IDs begin where node IDs end (e.g., with 6 nodes and 2 clients, client IDs are `6` and `7`).

## Drivers

The driver controls how packets are formatted, encrypted, and routed.

| Driver | Timestamp | Encryption | Cover traffic | Reliability | Manual mode |
|--------|-----------|------------|---------------|-------------|-------------|
| `simple` | Discrete (u32 tick counter) | None | No | No | Yes |
| `sphinx` | Wall-clock (`Instant`) | Full Sphinx | Yes (Poisson) | SURB ACKs | No |
| `discrete-sphinx` | Discrete (u32 tick counter) | Full Sphinx | Yes (Poisson) | SURB ACKs | Yes |

**`simple`** — Each packet is a fixed 64-byte frame (16-byte UUID + 48-byte payload). Nodes forward to `node_id + 1`. No cryptography. Best for sanity-checking the topology and observing raw packet flow.

**`sphinx`** — Uses `nym_sphinx::SphinxPacket` for full onion encryption. Clients build a 3-hop route, generate a SURB ACK reliability layer, and run two Poisson cover-traffic loops. Per-hop delays are extracted from the decrypted packet and scheduled using real wall-clock time. Automatic mode only.

**`discrete-sphinx`** — Same Sphinx encryption, SURB ACKs, and cover traffic as `sphinx`, but uses a u32 tick counter instead of wall-clock time (1 tick = 1 ms). This makes timing deterministic and compatible with `--manual` mode. Default driver.

## Tick Mechanics

Each tick runs four phases across all participants:

1. **Clients** — every client drains its app socket, runs new payloads through the wrapping pipeline, processes any inbound mix packets, and forwards queued packets whose scheduled timestamp is due.
2. **Nodes — incoming** — every node drains its UDP socket (non-blocking) and buffers received packets.
3. **Nodes — processing** — buffered packets pass through the mixing pipeline. For Sphinx nodes, this means decryption and routing extraction. Each processed packet is queued with a scheduled dispatch timestamp.
4. **Nodes — outgoing** — packets whose timestamp ≤ current tick are serialised and sent via UDP to the next hop.

In manual mode, the node state is pretty-printed between phases 2 and 3, and again between 3 and 4 (unless `--no-display-state` is set).

## Speed Controls

**Tick duration** (`--tick-duration-ms`) controls how fast the simulation runs:

- `0` — maximum speed, no sleep between ticks
- `1` (default) — roughly real-time for discrete drivers
- Any value `N > 1` — slows the simulation down linearly; in practice a value of `N` will make the simulation `N` times slower than real time

**Manual mode** (`--manual`) pauses after every tick and waits for ENTER. Completely deterministic — no timing overhead, step through packet sequences one tick at a time. Only available with the `simple` and `discrete-sphinx` drivers.

**Discrete vs wall-clock timestamps** — Discrete (u32) timestamps have minimal overhead and allow the simulation to run faster than real time. Wall-clock (`Instant`) timestamps tie delays to real elapsed time, which is more realistic but limits simulation speed.

## Topology File

`topology.json` is generated by `init-topology` and consumed by `run` and `mix-client`.

```json
{
  "nodes": [
    {
      "node_id": 0,
      "socket_address": "127.0.0.1:9000",
      "reliability": 100,
      "sphinx_private_key": "<bs58-encoded X25519 key>"
    }
  ],
  "clients": [
    {
      "client_id": 6,
      "mixnet_address": "127.0.0.1:9506",
      "app_address": "127.0.0.1:9606"
    }
  ]
}
```

The `reliability` field is reserved for future use.

## Logging

Set `RUST_LOG` to control verbosity:

```bash
RUST_LOG=debug cargo run --bin nym-mix-sim -- run
RUST_LOG=warn  cargo run --bin nym-mix-sim -- run   # quiet
```

Default level is `info`. Logs go to stderr; received message content goes to stdout.

## Example: Manual Sphinx Walk-Through

```bash
# Terminal 1 — run in manual mode, one tick at a time
cargo run --bin nym-mix-sim -- run --driver discrete-sphinx --manual

# Terminal 2 — send a message from client 6 to client 7
cargo run --bin mix-client -- --src 6 --dst 7
> hello

# Back in Terminal 1, press ENTER to advance each tick and observe
# the encrypted packet hop through each node
```
