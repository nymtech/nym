# A service provider on the Lewes Protocol

What a provider embedded in a nym-node uses instead of a mixnet client: a pipeline pair that peels
what arrives and routes what it sends, exchanging sphinx with its host over channels rather than a
socket.

A standalone provider has none of this. Which one it is comes from `ServiceProviderMode` alone
(`../mode.rs`): `Standalone`, or `Embedded(EmbeddedSetup)`, whose `start` hands back the transceiver
the mixnet client is built with *and* the LP data plane - both, or neither.

## The two links

```text
                      NR / IPR / authenticator loop
                                 │    ProviderLink  ← the provider's side
        inbound: Vec<u8> ▼       │    ▲ outbound: ServiceProviderReply
            (plaintext)          │    │      (plaintext + Recipient)
                    ───────────────────── tokio::mpsc, PROVIDER_CHANNEL_BUFFER
                                 │
                 SpInbound ──────┴────── SpOutbound     (blocking threads, not tasks)
           dispatcher + N workers        one scheduler
              SpInboundPipeline        SpOutboundPipeline
                                 │    PipelineLink  ← the pipelines' side
              from_gateway ▼     │    ▲ to_gateway
        (wrapped packets)        │    │      (LpFrame)
                    ───────────────────── std::sync::mpsc, PROVIDER_CHANNEL_DEPTH
                                 │    GatewayLink   ← the host's side
                       the host node's LP data plane
```

Below the pipelines the channels are **synchronous** - every end is a blocking pipeline worker or a
tick loop that only ever `try_`s, so there is no runtime involved and a full channel drops, as the
socket does. Above them they are tokio channels, because the provider's loop is async.

## Which channel is where

Each link is one struct per side, named for whose side it is - so the name says which half you are
holding, and the field says which way its bytes go. Every field is named absolutely, never relative
to its holder, so the two sides of a link read the same way round.

| Side | Field | Carries | Held by |
|---|---|---|---|
| `GatewayLink` | `to_provider` | wrapped packets (`Vec<u8>`) | the gateway, in `LocalEmbeddedClientHandle`; written through `EmbeddedServiceProviders::deliver_sp_payload` |
| | `from_provider` | `LpFrame` | the node, as `StartedServiceProvider::lp_output_rx`, drained into `WorkerJob::Local` |
| `PipelineLink` | `from_gateway` | wrapped packets (`Vec<u8>`) | `SpInbound`, which dispatches them to its workers |
| | `to_gateway` | `LpFrame` | `SpOutbound` |
| `ProviderLink` | `inbound` | plaintext (`Vec<u8>`) | the provider's loop |
| | `outbound` | `ServiceProviderReply` - plaintext + `Recipient` | the provider's loop |

`ProviderLink` has no counterpart struct: the pipelines' side of that link stays inside
`SpLpDataSetup` and nothing hands it out.

One link per provider rather than one shared: each gets its own backpressure, and the bandwidth
check the forward path still owes can tell whose traffic it is looking at.

`SpMessageRouterBuilder` (in the gateway crate) opens the link and hands each side out once -
`embedded_setup` gives the provider its `PipelineLink`, the gateway keeps the `GatewayLink`.

## What the pipelines do

**Inbound** (`handler/pipeline/inbound.rs`) - no framing to strip and no crypto to undo, because the
gateway already did both: peel the final sphinx layer with the provider's own x25519 key, reassemble
fragments, hand over `Vec<u8>`. Nothing of the legacy delivery machinery is involved - no
`ReceivedMessagesBuffer`, no `ReconstructedMessage`, no `PacketRouter`.

It runs on a pool, because peeling is the expensive half - an x25519 operation and an AEAD open per
packet - and packets are independent. A dispatcher hands them round-robin to `inbound_workers`
threads, each holding a *clone* of the pipeline; what the clones share is the reassembler, behind
its own lock, so whichever worker receives a message's last fragment completes it. Each worker's
queue is bounded and a packet that fits nowhere is dropped, as one arriving at a full channel from
the gateway is. Messages can reach the provider out of order - they already could, since LP has no
reliability and datagrams arrive as they arrive.

**Outbound** (`handler/pipeline/outbound.rs`) - chunk the message, route each fragment through
sphinx, emit **one whole `LpFrame` per packet**. It does not fragment: a client splits a sphinx
packet across two frames to fit an MTU, and this frame never meets a wire. The gateway fragments on
egress, because it is the one that does.

No pool here. This direction is not a stream of independent jobs but a scheduler - a 1ms tick and a
release buffer, which exist for the reliability and cover-traffic stages still to come. Sharding it
would give N partial buffers and a fuzzier notion of "due"; when those stages land, the buffer wants
one owner more than it wants parallelism.

> Its `frame_size()` must still come out at the real client's, `MTU - EncryptedLpPacket::OVERHEAD -
> LpFrameHeader::SIZE`, even though nothing here is MTU-bound: `chunk_size()` derives from it, and
> getting it wrong makes this provider emit sphinx packets of a visibly different size from every
> other client on the network.

## The seam, and `legacy`

`ProviderLink` is where the transport stops: what comes out is plaintext and nothing more. What a
provider does with it is its own, and is the same code whichever transport delivered it.

The reply still has to leave the way its request arrived, and plaintext cannot say which way that
was - so the **loop tags it**, which is free, because a `select!` arm already knows which channel it
read from: `true` at the mixnet client's arm, `false` at the LP one. From there `legacy` travels
through `ServiceProvider::on_request` and `handle_provider_data_request` to whatever builds the
return address. That is what lets one provider serve both transports at once.

LP carries no SURBs. A reply that needs one cannot leave this way, and each provider drops it at a
marked site rather than answering on a transport nobody is listening on.

## Files

| | |
|---|---|
| `../mode.rs` | `ServiceProviderMode`, `EmbeddedSetup` - the single embedded-or-standalone signal, and what the host gives: channels, topology, worker count |
| `mod.rs` | `gateway_link()` and its two sides, `PipelineLink` and `GatewayLink` |
| `handler/mod.rs` | `SpLpDataSetup` (built, then started), `ProviderLink` |
| `handler/inbound.rs` | the dispatcher and its pool of peeling workers |
| `handler/outbound.rs` | the 1ms tick, release buffer, `ServiceProviderReply` |
| `handler/pipeline/` | the two pipelines and their round-trip tests |
| `error.rs` | `LpProviderError` |
