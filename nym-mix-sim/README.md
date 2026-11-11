# nym-mix-sim

A tick-based simulator for the Nym mixnet, intended for local testing and experimentation. It models a small network of mix nodes and clients exchanging UDP packets on localhost, allowing you to observe packet flow, experiment with different drivers, and debug routing behaviour step by step.

## Overview

The simulator runs a configurable number of mix nodes and clients on localhost, each bound to its own loopback address. Time advances in **ticks** — each tick runs the client phase, then drains incoming sockets, processes packets through the mixing pipeline, and dispatches outgoing packets.

Two binaries are provided:

| Binary | Purpose |
|--------|---------|
| `nym-mix-sim` | Main simulator: topology generation and tick-loop execution |
| `mix-client` | Standalone tool to inject messages into a running simulation |

## Quick Start

```bash
# 1. Generate a topology with 4 nodes (ids 1-4) and 2 clients (ids 5 and 6)
cargo run --bin nym-mix-sim -- init-topology

# 2. Run the simulation (automatic mode, 10ms ticks, default nym-node driver)
cargo run --bin nym-mix-sim -- run

# 3. In a separate terminal, send a message between the two clients
cargo run --bin mix-client -- --src 5 --dst 6
# Then type a message and press ENTER
```

### Loopback addresses

Every node and client binds its own `127.0.0.N` address. Linux routes all of `127.0.0.0/8` to the loopback interface out of the box; macOS only has `127.0.0.1`, so each further address needs an alias first:

```bash
# with the default topology, N runs from 2 to 6
sudo ifconfig lo0 alias 127.0.0.N
```

`init-topology` logs the range of addresses the generated topology needs.

## Commands

### `init-topology`

Generates a `topology.json` file describing nodes and clients.

```bash
cargo run --bin nym-mix-sim -- init-topology [OPTIONS]
```

| Option | Default | Description |
|--------|---------|-------------|
| `-n, --nodes <N>` | `4` | Number of mix nodes (at least 4, one per role) |
| `-c, --clients <N>` | `2` | Number of clients |
| `-o, --output <PATH>` | `topology.json` | Output file path |

Ids start at 1: nodes get `1..=N` and clients the ids after them. A node with id `i` listens on `127.0.0.i:51264`; a client with id `i` gets two sockets, a mix-facing one on `127.0.0.i:9000` and an app-facing one on `127.0.0.i:9001`. Roles are assigned round-robin over `layer1`, `layer2`, `layer3` and `gateway`. Every node and client gets a freshly generated X25519 key pair.

### `run`

Starts the simulation loop.

```bash
cargo run --bin nym-mix-sim -- run [OPTIONS]
```

| Option | Default | Description |
|--------|---------|-------------|
| `-t, --topology <PATH>` | `topology.json` | Topology file to load |
| `--driver <DRIVER>` | `nym-node` | Simulation driver: `simple`, `sphinx` or `nym-node` (see below) |
| `-d, --tick-duration-ms <MS>` | `10` | Milliseconds per tick |
| `-m, --manual` | off | Enable manual stepping mode (ENTER per tick) |
| `--no-display-state` | off | Suppress the per-phase network display in manual mode |

The topology is rejected unless it has at least four nodes and every role is held by at least one of them.

### `mix-client`

Injects messages into a running simulation from stdin.

```bash
cargo run --bin mix-client -- --src <ID> --dst <ID> [--topology <PATH>]
```

Reads lines from stdin and sends each to the app socket of client `--src`, which routes it through the mix network to client `--dst`. Client ids begin after the node ids (e.g. with 4 nodes and 2 clients, client ids are `5` and `6`).

## Drivers

The driver controls how packets are formatted, encrypted, and routed. Every driver uses wall-clock `Instant` timestamps and supports `--manual`.

| Driver | Encryption | Cover traffic | Reliability |
|--------|------------|---------------|-------------|
| `simple` | None | No | No |
| `sphinx` | Full Sphinx | Yes (Poisson) | SURB ACKs |
| `nym-node` | Sphinx inside LP | No | No |

**`simple`** — Each packet is a fixed 64-byte frame (16-byte UUID + 48-byte payload). Node `i` forwards to node `i + 1`, and the last node to the client with the next id, regardless of the destination asked for. No cryptography. Best for sanity-checking the topology and observing raw packet flow.

**`sphinx`** — Uses `nym_sphinx::SphinxPacket` for full onion encryption. Clients build a 3-hop route (a random first hop plus two more, drawn from all nodes regardless of role), generate a SURB ACK reliability layer, and run two Poisson cover-traffic loops. Per-hop delays are drawn from an exponential distribution with a 50 ms mean.

**`nym-node`** — Every node runs the real nym-node data pipeline, and every client the real client LP pipeline, so packets are Sphinx packets carried in LP. LP sessions between every pair of nodes, and between every client and every node, are established up front with the real handshakes over in-memory channels. A client sends each message through a randomly drawn gateway; the route then crosses the three mix layers to the recipient's gateway. Default driver.

## Tick Mechanics

Each tick runs four phases across all participants:

1. **Clients** — every client drains its app socket, runs new payloads through the wrapping pipeline, and unwraps any inbound mix packets.
2. **Nodes — incoming** — every node drains its UDP socket (non-blocking) and buffers received packets.
3. **Nodes — processing** — buffered packets pass through the mixing pipeline. For Sphinx nodes, this means decryption and routing extraction. Each processed packet is queued with a scheduled dispatch timestamp.
4. **Outgoing** — every node, then every client, sends the packets whose timestamp ≤ current tick to their next hop.

In manual mode, the network state is drawn after phase 2 and again after phase 3 (unless `--no-display-state` is set).

## Speed Controls

**Tick duration** (`--tick-duration-ms`) controls how fast the simulation runs:

- In automatic mode, the simulation sleeps this long between ticks and each tick is stamped with the current wall-clock time. `0` runs ticks back to back.
- In manual mode, each ENTER advances the simulated clock by exactly this much, whatever time has really passed.

**Manual mode** (`--manual`) pauses before every tick and waits for ENTER, so packet sequences can be stepped through one tick at a time.

## Topology File

`topology.json` is generated by `init-topology` and consumed by `run` and `mix-client`.

```json
{
  "nodes": [
    {
      "node_id": 1,
      "socket_address": "127.0.0.1:51264",
      "role": "layer1",
      "reliability": 100,
      "sphinx_private_key": "<bs58-encoded X25519 key>"
    }
  ],
  "clients": [
    {
      "client_id": 5,
      "mixnet_address": "127.0.0.5:9000",
      "app_address": "127.0.0.5:9001",
      "sphinx_private_key": "<bs58-encoded X25519 key>"
    }
  ]
}
```

`role` is one of `layer1`, `layer2`, `layer3` or `gateway`. The `reliability` field is reserved for future use.

## Logging

Set `RUST_LOG` to control verbosity:

```bash
RUST_LOG=debug cargo run --bin nym-mix-sim -- run
RUST_LOG=warn  cargo run --bin nym-mix-sim -- run   # quiet
```

Default level is `info`. Logs go to stderr, including the content of received messages, which is logged at `info`. The manual-mode network display goes to stdout.

## Example: Manual Walk-Through

```bash
# Terminal 1 — run in manual mode, one tick at a time
cargo run --bin nym-mix-sim -- run --manual

# Terminal 2 — send a message from client 5 to client 6
cargo run --bin mix-client -- --src 5 --dst 6
> hello

# Back in Terminal 1, press ENTER to advance each tick and observe
# the encrypted packet hop through each node
```
