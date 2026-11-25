# A service provider on the Lewes Protocol

What a provider embedded in a nym-node uses instead of a mixnet client: a pipeline pair that peels
what arrives and routes what it sends, exchanging sphinx with its node over channels rather than a
socket.

A standalone provider has none of this. Each provider has one entry point per case -
`run_standalone()` and `run_embedded(EmbeddedSetup)` - so whoever starts it says which. An embedded
provider gets both paths from its node (`../embedded.rs`): `EmbeddedSetup { lp, legacy }`, the LP
channels and, for now, the legacy transceiver its mixnet client is built with.

## The channels

```text
                          NR / IPR loop
                                 │    ProviderLink  ← the provider's side
  inbound: ReconstructedMessage ▼│    ▲ outbound: ServiceProviderReply
        (plaintext + sender tag) │    │      (plaintext + ReplyTarget)
                    ───────────────────── tokio::mpsc, PROVIDER_CHANNEL_BUFFER
                                 │
                 SpInbound ──────┴────── SpOutbound     (blocking threads, not tasks)
           dispatcher + N workers        one scheduler
              SpInboundPipeline        SpOutboundPipeline
                                 │
              from_node ▼        │    ▲ to_node
        (wrapped packets)        │    │      (LpFrame)
   std::sync::mpsc, one per provider    tokio::mpsc, shared by every provider
                                 │
                       the node's LP data plane
```

Below the pipelines the node and the provider exchange two kinds of thing, each on its own kind of
channel:

- **Sphinx to one provider** - `provider_input_channel()`, one per provider. Synchronous, because
  only blocking threads touch it: the node's data workers write it, the provider's dispatcher reads
  it.
- **Frames from every provider** - `provider_output_channel(providers)`, one for the whole node,
  each provider holding a clone of the sender. A tokio channel, because the node's end is read by
  the LP data handler's async event loop; the providers' ends are blocking tick loops that only ever
  `try_send`, which needs no runtime. Sized so each provider keeps the depth a channel of its own
  would give it - but shared, so one that floods it crowds out the others.

Every writer only ever `try_`s: a full channel drops, as the socket does. Above the pipelines the
channels are tokio, because the provider's loop is async.

## Which end is where

Names follow one vocabulary: the host is the **node**, directions are `to_` / `from_`.

| Channel | End | Carries | Held by |
|---|---|---|---|
| provider input | `ProviderInputSender` | wrapped packets (`Vec<u8>`) | the node: `StartedServiceProvider::lp_to_provider`, indexed in `EmbeddedServiceProviders` and written through `deliver_sp_payload` |
| | `ProviderInputReceiver` | | `EmbeddedLp::from_node`, then `SpInbound`, which dispatches to its workers |
| provider output | `ProviderOutputSender` | `LpFrame` | `EmbeddedLp::to_node`, then `SpOutbound` - one clone per provider |
| | `ProviderOutputReceiver` | | the node's LP data handler, as `from_providers`, fed into `WorkerJob::Local` |
| `ProviderLink` | `inbound` | `ReconstructedMessage` - plaintext + sender tag | the provider's loop |
| | `outbound` | `ServiceProviderReply` - plaintext + `ReplyTarget` (a `Recipient` or a sender tag) | the provider's loop |

`ProviderLink` has no counterpart struct: the pipelines' side of it stays inside `SpLpDataSetup` and
nothing hands it out.

`ServiceProviderBeingBuilt::new` (in the gateway crate) opens both paths for a provider, so the two
ends of each always match: the legacy transceiver through `SpMessageRouterBuilder`, and the LP path
by calling `provider_input_channel()` and cloning the node's shared output sender into the
provider's `EmbeddedLp`.

## What the pipelines do

**Inbound** (`handler/pipeline/inbound.rs`) - no framing to strip and no crypto to undo, because the
node already did both: peel the final sphinx layer with the provider's own x25519 key, reassemble
fragments, hand over plaintext. Nothing of the legacy delivery machinery is involved - no
`ReceivedMessagesBuffer`, no `PacketRouter`.

It runs on a pool, because peeling is the expensive half - an x25519 operation and an AEAD open per
packet - and packets are independent. A dispatcher puts them on one bounded queue that
`inbound_workers` threads pull from, so a packet goes to whichever is free. Each worker holds a
*clone* of the pipeline; what the clones share is the reassembler, behind its own lock, so whichever
worker receives a message's last fragment completes it. A packet that does not fit the queue is
dropped, as one arriving at a full channel from the node is. Messages can reach the provider out of
order - they already could, since LP has no reliability and datagrams arrive as they arrive.

**Outbound** (`handler/pipeline/outbound.rs`) - chunk the message, route each fragment through
sphinx, emit **one whole `LpFrame` per packet**. It does not fragment: a client splits a sphinx
packet across two frames to fit an MTU, and this frame never meets a wire. The node fragments on
egress, because it is the one that does.

No pool here. This direction is not a stream of independent jobs but a scheduler - a 1ms tick and a
release buffer, which exist for the reliability and cover-traffic stages still to come. Sharding it
would give N partial buffers and a fuzzier notion of "due"; when those stages land, the buffer wants
one owner more than it wants parallelism. The tick loop ends once the provider drops its side.

> Its `frame_size()` must still come out at the real client's, `MTU - EncryptedLpPacket::OVERHEAD -
> LpFrameHeader::SIZE`, even though nothing here is MTU-bound: `chunk_size()` derives from it, and
> getting it wrong makes this provider emit sphinx packets of a visibly different size from every
> other client on the network.

## The seam, and `RequestPath`

`ProviderLink` is where the transport stops, and it looks to the provider much like the mixnet
client: a `ReconstructedMessage` in, and a reply out to either an explicit `Recipient` or a sender
tag. What a provider does with it is its own, and is the same code whichever transport delivered it.

The reply still has to leave the way its request arrived, and plaintext cannot say which way that
was - so the **loop tags it**, which is free, because a `select!` arm already knows which channel it
read from: `RequestPath::Legacy` at the mixnet client's arm, `RequestPath::Lp` at the LP one. From
there the path travels through `ServiceProvider::on_request` and `handle_provider_data_request` to
whatever sends the reply. That is what lets one provider serve both transports at once.

A `ReplyTarget::Recipient` is routed through the topology like any other message: the network
requester uses it for socks5 connections that carry a `return_address`. A `ReplyTarget::SenderTag`
is meant to be answered through the SURBs the sender attached, as the mixnet client does - the IP
packet router only ever replies this way. LP carries no SURBs yet, so the inbound worker delivers
`sender_tag: None` and the outbound tick drops every reply by tag, both marked TODO.

## Files

| | |
|---|---|
| `../embedded.rs` | `EmbeddedSetup`, `EmbeddedLp`, the channel types and `provider_input_channel` / `provider_output_channel` - everything the node gives a provider |
| `mod.rs` | `start_lp_data_plane`, which builds and starts a provider's pipelines from its `EmbeddedLp` |
| `handler/mod.rs` | `SpLpDataSetup` (built, then started), `ProviderLink` |
| `handler/inbound.rs` | the dispatcher and its pool of peeling workers |
| `handler/outbound.rs` | the 1ms tick, release buffer, `ServiceProviderReply` |
| `handler/pipeline/` | the two pipelines and their round-trip tests |
| `error.rs` | `LpProviderError` |
