// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::clients::{InputOptions, PipelinePayload};
use crate::common::traits::{Framing, Transport, WireUnwrappingPipeline, WireWrappingPipeline};
use crate::{AddressedTimedData, TimedPayload};

/// Trait for splitting an incoming payload into timestamped chunks.
///
/// # Type Parameters
/// - `Ts`: Timestamp type associated with each produced `TimedPayload`.
///
/// # Parameters
/// - `input`: Raw payload to split into chunks.
/// - `chunk_size`: Maximum size of each chunk in bytes.
/// - `timestamp`: Timestamp to assign to the produced chunks.
///
/// # Returns
/// - A vector of `TimedPayload`s representing the chunked payload.
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

/// Trait for applying reliability encoding to a timed payload.
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the `TimedPayload`.
///
/// # Associated Constants
/// - `OVERHEAD_SIZE`: Number of additional bytes added by the reliability scheme.
///
/// # Parameters
/// - `input`: Payload to encode with the reliability mechanism.
/// # Returns
/// - A vector of `TimedPayload` containing the reliability-encoded data and potential retransmissions.
pub trait Reliability<Ts, Opts, NdId>
where
    Opts: InputOptions<NdId>,
{
    const OVERHEAD_SIZE: usize;
    fn reliable_encode(
        &mut self,
        input: Option<PipelinePayload<Ts, Opts, NdId>>,
        timestamp: Ts,
    ) -> Vec<PipelinePayload<Ts, Opts, NdId>>;
}

/// Trait for applying obfuscation to a timed payload.
/// If obfuscation is used, `obfuscate` should be called at every `Ts` not just the ones with input
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the `TimedPayload`.
pub trait Obfuscation<Ts, Opts, NdId>
where
    Opts: InputOptions<NdId>,
{
    /// Obfuscate a given timed payload
    /// # Parameters
    /// - `input`: Optional payload to obfusctate
    /// - `timestamp` : Current timestamp
    ///
    /// # Returns
    /// - An `Vec<TimedPayload>`, result of the obfuscation algorithm
    /// - The vector can be empty if there is nothing to return right away
    fn obfuscate(
        &mut self,
        input: Option<PipelinePayload<Ts, Opts, NdId>>,
        timestamp: Ts,
    ) -> Vec<PipelinePayload<Ts, Opts, NdId>>;
}

/// Trait for applying routing-security encryption to a timed payload.
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the `TimedPayload`.
///
/// # Associated Constants
/// - `OVERHEAD_SIZE`: Number of additional bytes added by the encryption scheme.
///
/// # Required Methods
/// - `encrypt`: Encrypt the given payload, returning a new `TimedPayload`.
///
/// # Provided Methods
/// - `nb_frames`: Number of transport frames that one encrypted payload expands
///   into; defaults to `1`.  Override when the encryption scheme (e.g. Sphinx)
///   produces multiple frames per input chunk.
pub trait RoutingSecurity<Ts, Opts, NdId>
where
    Opts: InputOptions<NdId>,
{
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
/// - `Fr`: Intermediate frame type produced by framing.
/// - `Pkt`: Final transport packet type produced by transport.
///
/// # Provided Methods
/// - `chunk_size`: Derived from `frame_size` (via [`WireWrappingPipeline`]) minus
///   routing-security and reliability overheads, accounting for `nb_frames` expansion.
/// - `process`: Runs the full pipeline in order:
///   chunk → reliability encode → obfuscate → encrypt → frame → transport.
pub trait ClientWrappingPipeline<Ts, Fr, Pkt, Opts, NdId>:
    Chunking<Ts, Opts, NdId>
    + Reliability<Ts, Opts, NdId>
    + Obfuscation<Ts, Opts, NdId>
    + RoutingSecurity<Ts, Opts, NdId>
    + WireWrappingPipeline<Ts, Fr, Pkt, NdId>
where
    Ts: Clone,
    NdId: Clone,
    Opts: InputOptions<NdId>,
{
    fn chunk_size(&self, input_options: Opts) -> usize {
        // Frame size comes from WireWrappingPipeline
        let mut chunk_size = self.frame_size();

        if input_options.routing_security() {
            chunk_size =
                chunk_size * self.nb_frames() - <Self as RoutingSecurity<_, _, _>>::OVERHEAD_SIZE;
        }

        if input_options.reliability() {
            chunk_size -= <Self as Reliability<_, _, _>>::OVERHEAD_SIZE;
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

        // Even if we had nothing go into the reliablity stage, we need to catch potential retransmissions
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
            .flat_map(|payload| self.wire_wrap(payload.into()))
            .collect::<Vec<_>>()
    }
}

/// Dyn-compatible mirror of [`ClientWrappingPipeline`].
///
/// All associated constants from the sub-traits are exposed as methods so the
/// trait can be used as `dyn DynClientWrappingPipeline<Ts, Fr, Pkt>`, erasing the
/// concrete pipeline type while keeping `Ts`, `Fr`, and `Pkt`.
///
/// Implement [`ClientWrappingPipeline`] on your concrete type; the blanket impl
/// below provides `DynClientWrappingPipeline` for free.
pub trait DynClientWrappingPipeline<Ts, Fr, Pkt, Opts, NdId> {
    fn packet_size(&self) -> usize;

    // --- overhead accessors (mirrors of the supertrait associated constants) ---
    fn framing_overhead(&self) -> usize;
    fn transport_overhead(&self) -> usize;
    fn reliability_overhead(&self) -> usize;
    fn routing_overhead(&self) -> usize;
    fn nb_frames(&self) -> usize;

    // --- sizing helpers ---
    fn frame_size(&self) -> usize;

    fn chunk_size(&self, input_options: Opts) -> usize;

    fn process(
        &mut self,
        input: Option<(Vec<u8>, Opts)>,
        timestamp: Ts,
    ) -> Vec<AddressedTimedData<Ts, Pkt, NdId>>;
}

impl<T, Ts, Fr, Pkt, Opts, NdId> DynClientWrappingPipeline<Ts, Fr, Pkt, Opts, NdId> for T
where
    Ts: Clone,
    NdId: Clone,
    Opts: InputOptions<NdId>,
    T: ClientWrappingPipeline<Ts, Fr, Pkt, Opts, NdId>,
{
    fn packet_size(&self) -> usize {
        WireWrappingPipeline::packet_size(self)
    }

    fn framing_overhead(&self) -> usize {
        <T as Framing<_, _, _>>::OVERHEAD_SIZE
    }

    fn transport_overhead(&self) -> usize {
        <T as Transport<_, _, _, _>>::OVERHEAD_SIZE
    }

    fn reliability_overhead(&self) -> usize {
        <T as Reliability<_, _, _>>::OVERHEAD_SIZE
    }

    fn routing_overhead(&self) -> usize {
        <T as RoutingSecurity<_, _, _>>::OVERHEAD_SIZE
    }

    fn nb_frames(&self) -> usize {
        <T as RoutingSecurity<_, _, _>>::nb_frames(self)
    }

    fn frame_size(&self) -> usize {
        <T as WireWrappingPipeline<_, _, _, _>>::frame_size(self)
    }

    fn chunk_size(&self, input_options: Opts) -> usize {
        <T as ClientWrappingPipeline<_, _, _, _, _>>::chunk_size(self, input_options)
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
/// blank [`process_unwrapped`] step that the implementor fills in (routing-security
/// decrypt, reliability decode, chunk reassembly, etc.).
///
/// # Type Parameters
/// - `Ts`: Timestamp type.
/// - `Pkt`: Transport packet type consumed as input.
///
/// # Associated Types
/// - `Frame`: Intermediate frame type produced by the transport unwrap.
///
///
/// # Required Methods
/// - `process_unwrapped`: Called with the reassembled payload and its message kind
///   once a complete message is available. Returns the decoded application bytes,
///   or `None` if reassembly is still in progress.
///
/// # Provided Methods
/// - `unwrap`: Strips the wire layers via [`WireUnwrappingPipeline::wire_unwrap`],
///   then delegates to `process_unwrapped`.
pub trait ClientUnwrappingPipeline<Ts, Fr, Pkt, Mk>:
    WireUnwrappingPipeline<Ts, Fr, Pkt, Mk>
where
    Ts: Clone,
{
    fn process_unwrapped(&mut self, payload: TimedPayload<Ts>, kind: Mk) -> Option<Vec<u8>>;

    fn unwrap(&mut self, input: Pkt, timestamp: Ts) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(self
            .wire_unwrap(input, timestamp)?
            .and_then(|(payload, kind)| self.process_unwrapped(payload, kind)))
    }
}
