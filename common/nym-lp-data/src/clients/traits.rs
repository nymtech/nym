// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::PipelinePayload;
use crate::clients::InputOptions;
use crate::common::traits::{WireUnwrappingPipeline, WireWrappingPipeline};
use crate::{AddressedTimedData, TimedPayload};

/// Trait for splitting an incoming payload into timestamped chunks.
///
/// # Type Parameters
/// - `Ts`: Timestamp type associated with each produced [`PipelinePayload`].
/// - `Opts`: Per-message pipeline options (must implement [`InputOptions`]).
/// - `NdId`: Addressing type for the next-hop destination.
///
/// # Required Methods
/// - `chunked`: Split `input` into chunks of at most `chunk_size` bytes, tagging
///   each chunk with `timestamp` and `input_options`.  Returns one
///   [`PipelinePayload`] per chunk, ready to be fed through the rest of the
///   pipeline.
pub trait Chunking<Ts, Opts, NdId>
where
    Opts: InputOptions<NdId>,
{
    fn chunked(
        &mut self,
        input: Vec<u8>,
        input_options: Opts,
        chunk_size: usize,
        timestamp: Ts,
    ) -> Vec<PipelinePayload<Ts, Opts, NdId>>;
}

/// Trait for applying reliability encoding (e.g. SURB ACKs, retransmissions) to
/// a timed payload.
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the [`PipelinePayload`].
/// - `Opts`: Per-message pipeline options.
/// - `NdId`: Addressing type for the next-hop destination.
///
/// # Associated Constants
/// - `OVERHEAD_SIZE`: Number of additional bytes added by the reliability scheme.
///
/// # Required Methods
/// - `reliable_encode`: Encode `input` with the reliability mechanism.  When
///   `input` is `None`, the method is still called every tick so the layer can
///   emit pending retransmissions or scheduled control packets.
pub trait Reliability<Ts, Opts, NdId> {
    const OVERHEAD_SIZE: usize;
    fn reliable_encode(
        &mut self,
        input: Option<PipelinePayload<Ts, Opts, NdId>>,
        timestamp: Ts,
    ) -> Vec<PipelinePayload<Ts, Opts, NdId>>;
}

/// Trait for applying obfuscation (cover traffic, traffic shaping) to a timed payload.
///
/// When obfuscation is enabled, `obfuscate` must be called on every tick — not
/// only on ticks that carry input — so the layer can produce cover traffic on
/// schedule even when the application has nothing to send.
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the [`PipelinePayload`].
/// - `Opts`: Per-message pipeline options.
/// - `NdId`: Addressing type for the next-hop destination.
pub trait Obfuscation<Ts, Opts, NdId> {
    /// Obfuscate `input` at the given `timestamp`.
    ///
    /// # Parameters
    /// - `input`: Payload to obfuscate, or `None` when the pipeline is ticking
    ///   with no real message available.
    /// - `timestamp`: Current timestamp.
    ///
    /// # Returns
    /// A `Vec` of obfuscated payloads, possibly empty when no packet is due to be
    /// emitted at this tick.
    fn obfuscate(
        &mut self,
        input: Option<PipelinePayload<Ts, Opts, NdId>>,
        timestamp: Ts,
    ) -> Vec<PipelinePayload<Ts, Opts, NdId>>;
}

/// Trait for applying routing-security encryption (e.g. Sphinx) to a timed payload.
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the [`PipelinePayload`].
/// - `Opts`: Per-message pipeline options.
/// - `NdId`: Addressing type for the next-hop destination.
///
/// # Associated Constants
/// - `OVERHEAD_SIZE`: Number of additional bytes added by the encryption scheme.
///
/// # Required Methods
/// - `encrypt`: Encrypt the given payload, returning a new [`PipelinePayload`].
///
/// # Provided Methods
/// - `nb_frames`: Number of transport frames that one encrypted payload expands
///   into; defaults to `1`.  Override when the encryption scheme (e.g. Sphinx)
///   produces multiple frames per input chunk.
pub trait RoutingSecurity<Ts, Opts, NdId> {
    const OVERHEAD_SIZE: usize;
    fn nb_frames(&self) -> usize {
        1
    }
    fn encrypt(
        &mut self,
        input: PipelinePayload<Ts, Opts, NdId>,
    ) -> PipelinePayload<Ts, Opts, NdId>;
}

/// Full client-side outbound message pipeline.
///
/// Composes all six processing stages — [`Chunking`], [`Reliability`],
/// [`Obfuscation`], [`RoutingSecurity`], and the shared [`WireWrappingPipeline`]
/// (framing + transport) — into a single `process` call that takes a raw byte
/// payload and returns a list of timestamped transport packets ready for sending.
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried through the pipeline.
/// - `Pkt`: Final transport packet type produced by transport.
/// - `Opts`: Per-message pipeline options (must implement [`InputOptions`]).
/// - `NdId`: Addressing type for the next-hop destination.
///
/// # Provided Methods
/// - `chunk_size`: Derived from `frame_size` (via [`WireWrappingPipeline`]) minus
///   routing-security and reliability overheads, accounting for `nb_frames` expansion.
/// - `process`: Runs the full pipeline in order:
///   chunk → reliability encode → obfuscate → encrypt → frame → transport.
pub trait ClientWrappingPipeline<Ts, Pkt, Opts, NdId>:
    Chunking<Ts, Opts, NdId>
    + Reliability<Ts, Opts, NdId>
    + Obfuscation<Ts, Opts, NdId>
    + RoutingSecurity<Ts, Opts, NdId>
    + WireWrappingPipeline<Ts, Pkt, Opts, NdId>
where
    Ts: Clone,
    NdId: Clone,
    Opts: InputOptions<NdId>,
{
    fn chunk_size(&self, input_options: Opts) -> usize {
        // Frame size comes from WireWrappingPipeline
        let mut chunk_size = self.frame_size();

        if input_options.routing_security() {
            // SAFETY : While this CAN technically fail, it means that something is wrong in the code and it's pointless to continue anyway
            #[allow(clippy::expect_used)]
            let pre_security_chunk_size = (chunk_size * self.nb_frames())
                .checked_sub(<Self as RoutingSecurity<_, _, _>>::OVERHEAD_SIZE)
                .expect("not enough room in a packet for routing security overhead");
            chunk_size = pre_security_chunk_size;
        }

        if input_options.reliability() {
            // SAFETY : While this CAN technically fail, it means that something is wrong in the code and it's pointless to continue anyway
            #[allow(clippy::expect_used)]
            let pre_reliability_chunk_size = chunk_size
                .checked_sub(<Self as Reliability<_, _, _>>::OVERHEAD_SIZE)
                .expect("not enough room in a packet for reliability overhead");
            chunk_size = pre_reliability_chunk_size;
        }

        chunk_size
    }

    fn process(
        &mut self,
        input: Option<(Vec<u8>, Opts)>, // Optional to be able to tick the pipeline without input
        timestamp: Ts,
    ) -> Vec<AddressedTimedData<Ts, Pkt, NdId>> {
        let mut chunks = if let Some((input_data, input_options)) = input {
            self.chunked(
                input_data,
                input_options.clone(),
                self.chunk_size(input_options.clone()),
                timestamp.clone(),
            )
        } else {
            Vec::new()
        };

        // Reliability stage with chunks that needs reliability
        chunks = chunks
            .into_iter()
            .flat_map(|chunk| {
                if chunk.options.reliability() {
                    self.reliable_encode(Some(chunk), timestamp.clone())
                } else {
                    vec![chunk]
                }
            })
            .collect();

        // Even if we had nothing go into the reliability stage, we need to catch potential retransmissions
        // If we had, this should be a no-op, since it already has been called with the same timestamp
        chunks.append(&mut self.reliable_encode(None, timestamp.clone()));

        chunks = chunks
            .into_iter()
            .flat_map(|chunk| {
                if chunk.options.obfuscation() {
                    self.obfuscate(Some(chunk), timestamp.clone())
                } else {
                    vec![chunk]
                }
            })
            .collect();

        // Even if we had nothing go into the obfuscation stage, we need to catch potential cover traffic
        // If we had, this should be a no-op, since it already has been called with the same timestamp
        chunks.append(&mut self.obfuscate(None, timestamp.clone()));

        chunks = chunks
            .into_iter()
            .map(|chunk| {
                if chunk.options.routing_security() {
                    self.encrypt(chunk)
                } else {
                    chunk
                }
            })
            .collect();

        chunks
            .into_iter()
            .flat_map(|payload| self.wire_wrap(payload))
            .collect::<Vec<_>>()
    }
}

/// Dyn-compatible mirror of [`ClientWrappingPipeline`].
///
/// All associated constants from the sub-traits are exposed as methods so the
/// trait can be used as `dyn DynClientWrappingPipeline<Ts, Pkt, Opts, NdId>`,
/// erasing the concrete pipeline type while keeping `Ts`, `Pkt`, `Opts`, and
/// `NdId` visible.
///
/// Implement [`ClientWrappingPipeline`] on your concrete type; the blanket impl
/// below provides `DynClientWrappingPipeline` for free.
pub trait DynClientWrappingPipeline<Ts, Pkt, Opts, NdId> {
    /// On-wire size of an output packet in bytes.
    fn packet_size(&self) -> usize;

    /// Run the full client wrapping pipeline; see [`ClientWrappingPipeline::process`].
    fn process(
        &mut self,
        input: Option<(Vec<u8>, Opts)>,
        timestamp: Ts,
    ) -> Vec<AddressedTimedData<Ts, Pkt, NdId>>;
}

impl<T, Ts, Pkt, Opts, NdId> DynClientWrappingPipeline<Ts, Pkt, Opts, NdId> for T
where
    Ts: Clone,
    NdId: Clone,
    Opts: InputOptions<NdId>,
    T: ClientWrappingPipeline<Ts, Pkt, Opts, NdId>,
{
    fn packet_size(&self) -> usize {
        WireWrappingPipeline::packet_size(self)
    }

    fn process(
        &mut self,
        input: Option<(Vec<u8>, Opts)>,
        timestamp: Ts,
    ) -> Vec<AddressedTimedData<Ts, Pkt, NdId>> {
        ClientWrappingPipeline::process(self, input, timestamp)
    }
}

/// Full client-side inbound pipeline.
///
/// Combines the shared [`WireUnwrappingPipeline`] (transport + framing unwrap) with a
/// blank [`process_unwrapped`](Self::process_unwrapped) step that the implementor
/// fills in (routing-security decrypt, reliability decode, chunk reassembly, etc.).
///
/// # Type Parameters
/// - `Ts`: Timestamp type.
/// - `Pkt`: Transport packet type consumed as input.
/// - `Mk`: Message-kind marker returned alongside reassembled payloads.
///
/// # Required Methods
/// - `process_unwrapped`: Called with the reassembled payload and its message kind
///   once a complete message is available. Returns the decoded application bytes,
///   or `None` if reassembly is still in progress.
///
/// # Provided Methods
/// - `unwrap`: Strips the wire layers via [`WireUnwrappingPipeline::wire_unwrap`],
///   then delegates to `process_unwrapped`.
pub trait ClientUnwrappingPipeline<Ts, Pkt, Mk>: WireUnwrappingPipeline<Ts, Pkt, Mk>
where
    Ts: Clone,
{
    fn process_unwrapped(&mut self, payload: TimedPayload<Ts>, kind: Mk) -> Option<Vec<u8>>;

    fn unwrap(&mut self, input: Pkt, timestamp: Ts) -> Result<Option<Vec<u8>>, Self::Error> {
        Ok(self
            .wire_unwrap(input, timestamp)?
            .and_then(|(payload, kind)| self.process_unwrapped(payload, kind)))
    }
}
