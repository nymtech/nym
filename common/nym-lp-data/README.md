# nym-lp-data

Trait definitions and data structures for Lewes Protocol (LP) processing pipelines in the Nym mixnet.

This crate is a *vocabulary* crate — it defines the traits that clients and mix nodes implement to compose a packet-processing pipeline, plus a few generic data wrappers (`TimedData`, `AddressedTimedData`, `PipelineData`) that thread per-packet state through every stage. It contains no concrete cryptography, transport, or network code. A concrete implementation live in [`nym-mix-sim`](../../nym-mix-sim).

## Crate layout

| Module | Purpose |
|--------|---------|
| [`common`](src/common)     | Wire-layer traits ([`Framing`], [`FramingUnwrap`], [`Transport`], [`TransportUnwrap`]) and their composed supertraits ([`WireWrappingPipeline`], [`WireUnwrappingPipeline`]) shared by both clients and mixnodes, plus [`NoOpWireWrapper`] / [`NoOpWireUnwrapper`] marker traits for opting into a pass-through wire layer |
| [`clients`](src/clients)   | Client-side outbound/inbound pipeline traits: [`Chunking`], [`Reliability`], [`Obfuscation`], [`RoutingSecurity`], plus the supertraits [`ClientWrappingPipeline`] / [`ClientUnwrappingPipeline`], a `Pipeline` composition struct, no-op marker traits, and a tick-driven [`ClientWrappingPipelineDriver`] |
| [`mixnodes`](src/mixnodes) | Mixnode processing trait [`MixnodeProcessingPipeline`] (unwrap → mix → re-wrap) and a `Pipeline` composition struct |

[`Framing`]: src/common/traits.rs
[`FramingUnwrap`]: src/common/traits.rs
[`Transport`]: src/common/traits.rs
[`TransportUnwrap`]: src/common/traits.rs
[`WireWrappingPipeline`]: src/common/traits.rs
[`WireUnwrappingPipeline`]: src/common/traits.rs
[`NoOpWireWrapper`]: src/common/helpers.rs
[`NoOpWireUnwrapper`]: src/common/helpers.rs
[`Chunking`]: src/clients/traits.rs
[`Reliability`]: src/clients/traits.rs
[`Obfuscation`]: src/clients/traits.rs
[`RoutingSecurity`]: src/clients/traits.rs
[`ClientWrappingPipeline`]: src/clients/traits.rs
[`ClientUnwrappingPipeline`]: src/clients/traits.rs
[`ClientWrappingPipelineDriver`]: src/clients/driver.rs
[`MixnodeProcessingPipeline`]: src/mixnodes/traits.rs

## Core data types

```text
TimedData<Ts, D>           ──  pairs a value of type D with a timestamp Ts
TimedPayload<Ts>           ──  alias for TimedData<Ts, Vec<u8>>

AddressedTimedData<Ts, D, NdId>  ──  TimedData plus a destination address
AddressedTimedPayload<Ts, NdId>  ──  alias for AddressedTimedData<Ts, Vec<u8>, NdId>

PipelineData<Ts, D, Opts, NdId>  ──  TimedData plus per-message Opts
                                     (used inside the client wrapping pipeline)
PipelinePayload<Ts, Opts, NdId>  ──  alias for PipelineData<Ts, Vec<u8>, Opts, NdId>
```

`Ts` is the timestamp / tick-context type, `NdId` is the next-hop identifier type, and `Opts` is an [`InputOptions`](src/clients/mod.rs)-implementing per-message marker that toggles which optional pipeline stages run for a given payload (reliability, obfuscation, routing security).

## Client wrapping pipeline

The outbound client pipeline composes six stages, each represented by its own trait:

```text
                               Vec<u8>  ──▶  Chunking  ──▶  Reliability  ──▶  Obfuscation
                                                                                   │
                                                                                   ▼
        AddressedTimedData<Ts, Pkt, NdId>  ◀──  Transport  ◀──  Framing  ◀──  RoutingSecurity
```

[`ClientWrappingPipeline`] is the supertrait that ties them together and provides a default `process()` method which runs all six stages in order on every tick. Each stage is opt-in per message via the active [`InputOptions`].

### Pipeline tick semantics

`process()` is intended to be called on every tick (with or without an input payload):

- [`Reliability::reliable_encode`] is always called once with `Some(input)` (when present), then once more with `None` so that timer-driven retransmissions can fire even when no new payload arrived.
- [`Obfuscation::obfuscate`] follows the same pattern — once with the real input and once with `None` so that cover-traffic loops can fire on idle ticks.
- [`Chunking`] and [`RoutingSecurity`] only run when a payload is actually present.

This convention is what allows pipelines to support Poisson cover traffic and SURB-ACK retransmission without the caller having to know whether anything is in flight.

## Mixnode processing pipeline

The mixnode pipeline is simpler — three stages that consume a packet and emit zero or more re-wrapped output packets:

```text
Pkt  ──▶  WireUnwrappingPipeline  ──▶  mix  ──▶  WireWrappingPipeline  ──▶  Vec<AddressedTimedData<Ts, Pkt, NdId>>
        (TransportUnwrap +              ▲       (Framing + Transport)
         FramingUnwrap)                 │
                                        └──  implementor decrypts, routes,
                                              schedules delays, etc.
```

Implementors fill in `mix()`; everything else is provided by the [`MixnodeProcessingPipeline`] supertrait's default `process()`.

## Helpers

- **Client-stage no-op marker traits** ([`NoOpReliability`], [`NoOpRoutingSecurity`], [`NoOpObfuscation`] in [`clients/helpers.rs`](src/clients/helpers.rs)) — implement these to opt out of a pipeline stage with zero overhead. Useful for stub or testing pipelines.
- **Wire-layer no-op marker traits** ([`NoOpWireWrapper`], [`NoOpWireUnwrapper`] in [`common/helpers.rs`](src/common/helpers.rs)) — collapse the entire wire layer (framing + transport, or their inverses) to a pass-through. Use these when your packet type is already self-contained on the wire (e.g. a Sphinx packet) and needs no extra framing or transport header. `NoOpWireWrapper` requires `Pkt: From<Vec<u8>>`; `NoOpWireUnwrapper` requires `Pkt: Into<Vec<u8>>` and `Mk: Default`.
- **`Pipeline` composition structs** (in [`clients/types.rs`](src/clients/types.rs) and [`mixnodes/types.rs`](src/mixnodes/types.rs)) — generic structs that aggregate one component per pipeline stage and provide blanket impls of the relevant supertraits, so you can build a working pipeline by plugging in any combination of stage implementations.
- **[`ClientWrappingPipelineDriver`](src/clients/driver.rs)** — wraps a dyn-compatible client pipeline behind a tick-driven `tick(timestamp) -> Vec<(Pkt, NdId)>` interface, with an internal mpsc channel for application-supplied input payloads. Reads new input only when the internal buffer is empty so buffered packets do not stack additional latency on top.

[`NoOpReliability`]: src/clients/helpers.rs
[`NoOpRoutingSecurity`]: src/clients/helpers.rs
[`NoOpObfuscation`]: src/clients/helpers.rs
[`InputOptions`]: src/clients/mod.rs
[`Reliability::reliable_encode`]: src/clients/traits.rs
[`Obfuscation::obfuscate`]: src/clients/traits.rs

## Example users

[`nym-mix-sim`](../../nym-mix-sim) is the reference consumer: it ships two complete pipeline implementations (a pass-through `Simple*` family and a full Sphinx + Poisson + SURB-ACK family) on top of the traits defined here. See its source for end-to-end examples of implementing each pipeline stage.

The integration test under [`tests/integration`](tests/integration) wires together a small synthetic pipeline (`MockChunking`, `KcpReliability`, `SphinxSecurity`, `KekwObfuscation`, `LpFraming`, `LpTransport`) against the [`nym-lp`](../nym-lp) packet types — a useful starting point if you want to read a self-contained example of every trait being implemented.
