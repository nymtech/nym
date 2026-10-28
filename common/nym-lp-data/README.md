# nym-lp-data

Trait definitions, packet formats and data structures for Lewes Protocol (LP) data processing in the Nym network.

The crate has two halves:

- **Pipeline vocabulary**: the traits that clients, service providers and nym-nodes implement to compose a packet-processing pipeline, plus the generic wrappers (`TimedData`, `PipelineData`, `AddressedTimedData`) that carry per-packet state through every stage.
- **LP wire format**: the LP packet, header and frame types, the size budgets derived from the path MTU, protocol version negotiation, and frame fragmentation/reassembly.

It contains no concrete cryptography, session handling or network I/O. `EncryptedLpPacket` holds ciphertext but does not produce it; the AEAD and the sessions behind it live in [`nym-lp`](../nym-lp).

## Crate layout

| Module | Purpose |
|--------|---------|
| [`common`](src/common)               | Wire-layer traits ([`Framing`], [`FramingUnwrap`], [`Transport`], [`TransportUnwrap`]) and their composed supertraits ([`WireWrappingPipeline`], [`WireUnwrappingPipeline`]) shared by clients and nodes, plus the [`NoOpWireWrapper`] / [`NoOpWireUnwrapper`] marker traits for a pass-through wire layer |
| [`clients`](src/clients)             | Client-side stage traits ([`Chunking`], [`Reliability`], [`Obfuscation`], [`RoutingSecurity`]), the supertraits [`ClientWrappingPipeline`] / [`ClientUnwrappingPipeline`], the dyn-compatible [`DynClientWrappingPipeline`], a `Pipeline` composition struct, no-op marker traits, and the tick-driven [`ClientWrappingPipelineDriver`] |
| [`nymnodes`](src/nymnodes)           | Node processing trait [`NymNodeProcessingPipeline`]: reassemble a frame, mix it, re-frame the results |
| [`packet`](src/packet)               | LP wire types: [`LpPacket`] / [`EncryptedLpPacket`], [`LpHeader`] (`OuterHeader` + `InnerHeader`), [`LpFrame`] and its kinds and attributes, MTU-derived size constants, and [`version`] negotiation |
| [`fragmentation`](src/fragmentation) | Splitting an `LpFrame` into `FragmentedData` frames ([`fragment_lp_message`]) and reassembling them ([`MessageReconstructor`]) |

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
[`DynClientWrappingPipeline`]: src/clients/traits.rs
[`ClientWrappingPipelineDriver`]: src/clients/driver.rs
[`NymNodeProcessingPipeline`]: src/nymnodes/traits.rs
[`LpPacket`]: src/packet/mod.rs
[`EncryptedLpPacket`]: src/packet/mod.rs
[`LpHeader`]: src/packet/header.rs
[`LpFrame`]: src/packet/frame.rs
[`version`]: src/packet/version.rs
[`fragment_lp_message`]: src/fragmentation/fragment.rs
[`MessageReconstructor`]: src/fragmentation/reconstruction.rs

## Core data types

```text
TimedData<D>                  ──  a value of type D paired with an Instant timestamp
TimedPayload                  ──  alias for TimedData<Vec<u8>>

PipelineData<D, Opts, NdId>   ──  TimedData<D> plus per-message Opts and a destination NdId
PipelinePayload<Opts, NdId>   ──  alias for PipelineData<Vec<u8>, Opts, NdId>

AddressedTimedData<D, NdId>   ──  alias for PipelineData<D, (), NdId> (options dropped)
AddressedTimedPayload         ──  alias for AddressedTimedData<Vec<u8>>

PipelineInput<Opts>           ──  Vec<u8>, Opts and a SocketAddr, untimed: what a client wrapping pipeline is fed
```

`NdId` names the destination and defaults to `SocketAddr`. `Opts` is opaque per-message metadata that the pipeline threads through untouched; each implementation decides what it means. `PipelineData::into_addressed` drops the options once a stage no longer needs them, and `AddressedTimedData::with_options` adds them back.

The timestamp is when the packet is scheduled to go out, not when it was created: stages such as obfuscation may push it into the future, and the caller holds the packet until then.

## Client wrapping pipeline

The outbound client pipeline has six stages, each its own trait:

```text
     PipelineInput<Opts>       ──▶  Chunking  ──▶  Reliability  ──▶  Obfuscation
                                                                          │
                                                                          ▼
       Vec<AddressedTimedData<Pkt>>  ◀──  Transport  ◀──  Framing  ◀──  RoutingSecurity
```

[`ClientWrappingPipeline`] ties them together. Its provided `process()` runs every stage on every call; a pipeline that does not want a stage plugs in a no-op implementation (see [Helpers](#helpers)) whose overhead is zero. `process()` returns an error only when `Transport` fails, for example because there is no session for the destination.

The chunk size is derived rather than configured: `frame_size() * nb_frames()` minus the routing-security and reliability overheads, where `frame_size()` is `packet_size()` minus the transport and framing overheads.

[`DynClientWrappingPipeline`] is a dyn-compatible mirror of `ClientWrappingPipeline` with the transport error boxed. Every `ClientWrappingPipeline` gets it through a blanket impl.

### Tick semantics

`process()` is meant to be called on every tick, with or without input:

- [`Chunking`] runs only when there is input.
- [`Reliability::reliable_encode`] is called once per chunk. With no chunks it is called once with `None`, so timer-driven retransmissions can still fire.
- [`Obfuscation::obfuscate`] follows the same pattern, so cover traffic can be emitted on idle ticks.
- [`RoutingSecurity`] and the wire layer run on whatever obfuscation emitted, cover traffic included.

This is what lets a pipeline do Poisson cover traffic and SURB-ACK retransmission without the caller knowing whether anything is in flight.

## Client unwrapping pipeline

[`ClientUnwrappingPipeline`] strips the wire layer through [`WireUnwrappingPipeline::wire_unwrap`] (transport unwrap, then framing unwrap and reassembly), then hands the complete message and its kind to `process_unwrapped`, which the implementor fills in (decryption, reliability decoding, reassembly of chunks). Both steps return `None` while a message is still incomplete.

## Nym-node processing pipeline

[`NymNodeProcessingPipeline`] takes frames in and gives frames out:

```text
TimedData<Frame>  ──▶  FramingUnwrap  ──▶  mix  ──▶  Framing  ──▶  Vec<AddressedTimedData<Frame, NdId>>
                                            ▲
                                            └──  implementor decrypts, routes,
                                                 schedules delays, etc.
```

Implementors provide `mix()` and `frame_size()`; the provided `process()` does the rest. `mix()` also receives the frame's source address; frames handed over in-process (from a service provider the node hosts) pass an unspecified address.

The transport layer is deliberately outside this trait. `mix()` can schedule its output for later, and a transport that numbers packets on wrap (as LP's AEAD does, with a counter in the cleartext outer header) must wrap at send time. If it wrapped during processing, the counter order on the wire would reveal each packet's mixing delay, and late packets could fall outside the receiver's replay window. The caller unwraps before `process()` and wraps after it.

## LP packets

An LP packet is an `OuterHeader` (receiver index + counter, sent in the clear) followed by ciphertext covering the `InnerHeader` (protocol version) and one `LpFrame`. A frame is a 16-byte `LpFrameHeader` (kind + 14 bytes of kind-specific attributes) and its content. Frame kinds include `Opaque`, `Registration`, `Forward`, `SphinxStream`, `FragmentedData`, `SphinxPacket` and `ForwardSphinxPacket`.

Size budgets all derive from `PATH_MTU` (1500 by default) in [`packet/mod.rs`](src/packet/mod.rs):

| Constant | What it is |
|----------|------------|
| `MAX_UDP_PAYLOAD_SIZE`   | MTU minus IP and UDP headers |
| `MAX_FRAME_SIZE`         | UDP payload minus `EncryptedLpPacket::OVERHEAD` (headers + AEAD expansion) |
| `MAX_FRAME_PAYLOAD_SIZE` | Frame size minus `LpFrameHeader::SIZE`; the budget messages are chunked against |

Size a frame against `EncryptedLpPacket::OVERHEAD`, not `LpHeader::SIZE`: encryption expands the payload, so the plaintext header size underestimates the cost.

`FRAMES_PER_SPHINX_PACKET` is the number of frames one Sphinx packet spans (2). [`version`] holds the supported LP versions and `negotiate()`, which picks the version to speak with a peer.

## Fragmentation

[`fragment_lp_message`] splits an encoded `LpFrame` into `FragmentedData` frames. Each frame carries a random 64-bit message id, its index and the fragment count in its attributes. It is meant for the framing stage, not chunking, and fails if a message needs more than 255 fragments.

[`MessageReconstructor`] buffers fragments by message id and returns the original `LpFrame` once all of them have arrived. Duplicates are ignored. Incomplete messages are evicted once no fragment has arrived for the timeout (30 s by default); eviction runs on every insert and can also be triggered with `cleanup_stale_buffers`.

## Helpers

- **Client-stage no-op marker traits** ([`NoOpReliability`], [`NoOpRoutingSecurity`], [`NoOpObfuscation`] in [`clients/helpers.rs`](src/clients/helpers.rs)): implement one to get a pass-through stage with zero overhead.
- **Wire-layer no-op marker traits** ([`NoOpWireWrapper`], [`NoOpWireUnwrapper`] in [`common/helpers.rs`](src/common/helpers.rs)): collapse framing and transport (or their inverses) to a pass-through with `Infallible` errors, for packet types that are already self-contained on the wire. `NoOpWireWrapper` requires `Pkt: From<Vec<u8>>` and has a `PACKET_SIZE` of 1500 that implementors can override; `NoOpWireUnwrapper` requires `Pkt: Into<Vec<u8>>` and `Mk: Default`.
- **`Pipeline` composition struct** ([`clients/types.rs`](src/clients/types.rs)): holds one component per stage plus a `packet_size`, and implements every stage trait and `ClientWrappingPipeline` by delegating to them, so a pipeline can be assembled from independent stage implementations.
- **[`ClientWrappingPipelineDriver`]**: wraps a `DynClientWrappingPipeline` behind `tick(now) -> Vec<(Pkt, SocketAddr)>`, returning the packets whose timestamp is due. Application payloads go in through `input_sender()`, a zero-capacity (rendezvous) channel. A new payload is read only when no packets are buffered, so input does not queue up behind packets that are already waiting. Transport errors are logged, not returned.

[`NoOpReliability`]: src/clients/helpers.rs
[`NoOpRoutingSecurity`]: src/clients/helpers.rs
[`NoOpObfuscation`]: src/clients/helpers.rs
[`Reliability::reliable_encode`]: src/clients/traits.rs
[`Obfuscation::obfuscate`]: src/clients/traits.rs
[`WireUnwrappingPipeline::wire_unwrap`]: src/common/traits.rs

## Implementations

Production pipelines:

- **Clients**: `LpOutboundPipeline` / `LpInboundPipeline` in [`client-core`](../client-core/src/client/lp/data/handler/pipeline)
- **Service providers**: `SpOutboundPipeline` / `SpInboundPipeline` in [`service-providers/common`](../../service-providers/common/src/lp/handler/pipeline)
- **Nym-nodes**: `NymNodeDataPipeline` in [`nym-node`](../../nym-node/src/node/lp/data/handler/pipeline/nymnode.rs)

[`nym-mix-sim`](../../nym-mix-sim) is a self-contained simulator with two pipeline families on these traits: a pass-through `Simple*` family and a Sphinx family with Poisson cover traffic and SURB ACKs. It is the easiest place to read a complete implementation of each stage.

The integration test under [`tests/integration`](tests/integration) assembles a small pipeline from mock stages (`MockChunking`, `MockReliability`, `KekwObfuscation`, `MockSphinxSecurity`, `MockLpFraming`, `MockLpTransport`) through the `Pipeline` struct, producing this crate's `LpPacket`s.
