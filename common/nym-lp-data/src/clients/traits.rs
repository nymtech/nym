// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;
use std::time::Instant;

use crate::PipelinePayload;
use crate::common::traits::{Transport, WireUnwrappingPipeline, WireWrappingPipeline};
use crate::{AddressedTimedData, TimedPayload};

/// Trait for splitting an incoming payload into timestamped chunks.
///
/// # Type Parameters
/// - `Opts`: Opaque per-message metadata carried by each produced [`PipelinePayload`].
///
/// # Required Methods
/// - `chunked`: Split `input` (a [`PipelinePayload`] carrying the raw bytes,
///   per-message options, and destination) into chunks of at most `chunk_size`
///   bytes. Each output [`PipelinePayload`] inherits the input's options and
///   destination and is stamped with `timestamp`, ready to be fed through the
///   rest of the pipeline.
pub trait Chunking<Opts> {
    fn chunked(
        &mut self,
        input: PipelinePayload<Opts>,
        chunk_size: usize,
        timestamp: Instant,
    ) -> Vec<PipelinePayload<Opts>>;
}

/// Trait for applying reliability encoding (e.g. SURB ACKs, retransmissions) to
/// a timed payload.
///
/// # Type Parameters
/// - `Opts`: Opaque per-message metadata carried by the [`PipelinePayload`].
///
/// # Associated Constants
/// - `OVERHEAD_SIZE`: Number of additional bytes added by the reliability scheme.
///
/// # Required Methods
/// - `reliable_encode`: Encode `input` with the reliability mechanism.  When
///   `input` is `None`, the method is still called every tick so the layer can
///   emit pending retransmissions or scheduled control packets.
pub trait Reliability<Opts> {
    const OVERHEAD_SIZE: usize;
    fn reliable_encode(
        &mut self,
        input: Option<PipelinePayload<Opts>>,
        timestamp: Instant,
    ) -> Vec<PipelinePayload<Opts>>;
}

/// Trait for applying obfuscation (cover traffic, traffic shaping) to a timed payload.
///
/// When obfuscation is enabled, `obfuscate` must be called on every tick — not
/// only on ticks that carry input — so the layer can produce cover traffic on
/// schedule even when the application has nothing to send.
///
/// # Type Parameters
/// - `Opts`: Opaque per-message metadata carried by the [`PipelinePayload`].
pub trait Obfuscation<Opts> {
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
        input: Option<PipelinePayload<Opts>>,
        timestamp: Instant,
    ) -> Vec<PipelinePayload<Opts>>;
}

/// Trait for applying routing-security encryption (e.g. Sphinx) to a timed payload.
///
/// # Type Parameters
/// - `Opts`: Opaque per-message metadata carried by the [`PipelinePayload`].
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
pub trait RoutingSecurity<Opts> {
    const OVERHEAD_SIZE: usize;
    fn nb_frames(&self) -> usize;
    fn encrypt(&mut self, input: PipelinePayload<Opts>) -> PipelinePayload<Opts>;
}

/// Full client-side outbound message pipeline.
///
/// Composes all six processing stages — [`Chunking`], [`Reliability`],
/// [`Obfuscation`], [`RoutingSecurity`], and the shared [`WireWrappingPipeline`]
/// (framing + transport) — into a single `process` call that takes a raw byte
/// payload and returns a list of timestamped transport packets ready for sending.
///
/// Every stage runs unconditionally; a pipeline that does not want a given stage
/// composes a no-op implementation for it (see the `NoOp*` marker traits), whose
/// `OVERHEAD_SIZE` is `0`.
///
/// # Type Parameters
/// - `Pkt`: Final transport packet type produced by transport.
/// - `Opts`: Opaque per-message metadata threaded through the pipeline.
///
/// # Provided Methods
/// - `chunk_size`: Derived from `frame_size` (via [`WireWrappingPipeline`]) minus
///   routing-security and reliability overheads, accounting for `nb_frames` expansion.
/// - `process`: Runs the full pipeline in order:
///   chunk → reliability encode → obfuscate → encrypt → frame → transport.
pub trait ClientWrappingPipeline<Pkt, Opts>:
    Chunking<Opts>
    + Reliability<Opts>
    + Obfuscation<Opts>
    + RoutingSecurity<Opts>
    + WireWrappingPipeline<Pkt, Opts>
{
    fn chunk_size(&self) -> usize {
        // Frame size comes from WireWrappingPipeline
        // SAFETY : While this CAN technically fail, it means that something is wrong in the code and it's pointless to continue anyway
        #[allow(clippy::expect_used)]
        (self.frame_size() * self.nb_frames())
            .checked_sub(<Self as RoutingSecurity<_>>::OVERHEAD_SIZE)
            .expect("not enough room in a packet for routing security overhead")
            .checked_sub(<Self as Reliability<_>>::OVERHEAD_SIZE)
            .expect("not enough room in a packet for reliability overhead")
    }

    fn process(
        &mut self,
        input: Option<(Vec<u8>, Opts, SocketAddr)>, // Optional to be able to tick the pipeline without input
        timestamp: Instant,
    ) -> Result<Vec<AddressedTimedData<Pkt>>, <Self as Transport<Pkt>>::Error> {
        let chunk_size = self.chunk_size();
        let mut chunks = if let Some((input_data, input_options, next_hop)) = input {
            let input_payload =
                PipelinePayload::new(timestamp, input_data, input_options, next_hop);
            self.chunked(input_payload, chunk_size, timestamp)
        } else {
            Vec::new()
        };

        // Reliability stage
        chunks = if chunks.is_empty() {
            // Even if we had nothing go into the reliability stage, we need to catch potential retransmissions
            self.reliable_encode(None, timestamp)
        } else {
            chunks
                .into_iter()
                .flat_map(|chunk| self.reliable_encode(Some(chunk), timestamp))
                .collect()
        };

        // Obfuscation stage
        chunks = if chunks.is_empty() {
            // Even if we had nothing go into the obfuscation stage, we need to catch potential cover traffic
            self.obfuscate(None, timestamp)
        } else {
            chunks
                .into_iter()
                .flat_map(|chunk| self.obfuscate(Some(chunk), timestamp))
                .collect()
        };

        // Routing-security stage
        chunks = chunks
            .into_iter()
            .map(|chunk| self.encrypt(chunk))
            .collect();

        let mut packets = Vec::new();
        for payload in chunks {
            packets.extend(self.wire_wrap(payload)?);
        }

        Ok(packets)
    }
}

/// Dyn-compatible mirror of [`ClientWrappingPipeline`].
///
/// All associated constants from the sub-traits are exposed as methods so the
/// trait can be used as `dyn DynClientWrappingPipeline<Pkt, Opts>`, erasing the
/// concrete pipeline type while keeping `Pkt` and `Opts` visible.
///
/// Implement [`ClientWrappingPipeline`] on your concrete type; the blanket impl
/// below provides `DynClientWrappingPipeline` for free.
pub trait DynClientWrappingPipeline<Pkt, Opts> {
    /// On-wire size of an output packet in bytes.
    fn packet_size(&self) -> usize;

    /// Run the full client wrapping pipeline; see [`ClientWrappingPipeline::process`].
    ///
    /// The transport error is boxed because this trait erases the concrete pipeline, and with it
    /// the associated error type.
    fn process(
        &mut self,
        input: Option<(Vec<u8>, Opts, SocketAddr)>,
        timestamp: Instant,
    ) -> Result<Vec<AddressedTimedData<Pkt>>, Box<dyn std::error::Error + Send + Sync>>;
}

impl<T, Pkt, Opts> DynClientWrappingPipeline<Pkt, Opts> for T
where
    T: ClientWrappingPipeline<Pkt, Opts>,
    <T as Transport<Pkt>>::Error: std::error::Error + Send + Sync + 'static,
{
    fn packet_size(&self) -> usize {
        WireWrappingPipeline::packet_size(self)
    }

    fn process(
        &mut self,
        input: Option<(Vec<u8>, Opts, SocketAddr)>,
        timestamp: Instant,
    ) -> Result<Vec<AddressedTimedData<Pkt>>, Box<dyn std::error::Error + Send + Sync>> {
        ClientWrappingPipeline::process(self, input, timestamp).map_err(Into::into)
    }
}

/// Full client-side inbound pipeline.
///
/// Combines the shared [`WireUnwrappingPipeline`] (transport + framing unwrap) with a
/// blank [`process_unwrapped`](Self::process_unwrapped) step that the implementor
/// fills in (routing-security decrypt, reliability decode, chunk reassembly, etc.).
///
/// # Type Parameters
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
pub trait ClientUnwrappingPipeline<Pkt, Mk>: WireUnwrappingPipeline<Pkt, Mk> {
    fn process_unwrapped(&mut self, payload: TimedPayload, kind: Mk) -> Option<Vec<u8>>;

    fn unwrap(&mut self, input: Pkt, timestamp: Instant) -> Result<Option<Vec<u8>>, Self::Error> {
        Ok(self
            .wire_unwrap(input, timestamp)?
            .and_then(|(payload, kind)| self.process_unwrapped(payload, kind)))
    }
}
