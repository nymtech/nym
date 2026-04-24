// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::clients::types::StreamOptions;
use crate::common::traits::{Framing, Transport};
use crate::{TimedData, TimedPayload};

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
pub trait Chunking<Ts> {
    fn chunked(&self, input: Vec<u8>, chunk_size: usize, timestamp: Ts) -> Vec<TimedPayload<Ts>>;
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
/// - A `TimedPayload` containing the reliability-encoded data.
pub trait Reliability<Ts> {
    const OVERHEAD_SIZE: usize;
    fn reliable_encode(&self, input: TimedPayload<Ts>) -> TimedPayload<Ts>;
}

/// Trait for applying obfuscation to a timed payload.
/// If obfuscation is used, `obfuscate` should be called at every `Ts` not just the ones with input
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the `TimedPayload`.
pub trait Obfuscation<Ts> {
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
        input: Option<TimedPayload<Ts>>,
        timestamp: Ts,
    ) -> Vec<TimedPayload<Ts>>;

    /// Return the size of the inner timed payload buffer, to help with backpressure
    fn buffer_size(&self) -> usize;
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
pub trait RoutingSecurity<Ts> {
    const OVERHEAD_SIZE: usize;
    fn nb_frames(&self) -> usize {
        1
    }
    fn encrypt(&self, input: TimedPayload<Ts>) -> TimedPayload<Ts>;
}

/// Full client-side message pipeline.
///
/// Composes all six processing stages — [`Chunking`], [`Reliability`],
/// [`Obfuscation`], [`RoutingSecurity`], [`Framing`], and [`Transport`] — into
/// a single `process` call that takes a raw byte payload and returns a list of
/// timestamped transport packets ready for sending.
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried through the pipeline.
/// - `Fr`: Intermediate frame type produced by [`Framing`].
/// - `Pkt`: Final transport packet type produced by [`Transport`].
///
/// # Required Methods
/// - `packet_size`: Total on-wire size of the outputted transport packets in bytes.
///
/// # Provided Methods
/// - `frame_size`: Derived from `packet_size` minus transport and framing overheads.
/// - `chunk_size`: Derived from `frame_size` minus routing-security and reliability
///   overheads, accounting for `nb_frames` expansion.
/// - `process`: Runs the full pipeline in order:
///   chunk → reliability encode → obfuscate → encrypt → frame → transport.
pub trait ProcessingPipeline<Ts, Fr, Pkt>:
    Chunking<Ts>
    + Reliability<Ts>
    + Obfuscation<Ts>
    + RoutingSecurity<Ts>
    + Framing<Ts, Fr>
    + Transport<Ts, Fr, Pkt>
where
    Ts: Clone,
{
    fn packet_size(&self) -> usize;
    fn frame_size(&self) -> usize {
        self.packet_size()
            - <Self as Transport<_, _, _>>::OVERHEAD_SIZE
            - <Self as Framing<_, _>>::OVERHEAD_SIZE
    }

    fn chunk_size(&self, processing_options: StreamOptions) -> usize {
        // Frame size
        let mut chunk_size = self.frame_size();

        if processing_options.security {
            chunk_size =
                chunk_size * self.nb_frames() - <Self as RoutingSecurity<_>>::OVERHEAD_SIZE;
        }
        if processing_options.reliability {
            chunk_size -= <Self as Reliability<_>>::OVERHEAD_SIZE;
        }
        chunk_size
    }

    fn process(
        &mut self,
        input: Vec<u8>,
        processing_options: StreamOptions,
        timestamp: Ts,
    ) -> Vec<TimedData<Ts, Pkt>> {
        let mut chunks = self.chunked(
            input,
            self.chunk_size(processing_options),
            timestamp.clone(),
        );

        if processing_options.reliability {
            chunks = chunks
                .into_iter()
                .map(|chunk| self.reliable_encode(chunk))
                .collect();
        };

        if processing_options.obfuscation {
            // This needs to happen regarldess of if we took something as input
            if chunks.is_empty() {
                chunks = self.obfuscate(None, timestamp.clone());
            } else {
                chunks = chunks
                    .into_iter()
                    .flat_map(|chunk| self.obfuscate(Some(chunk), timestamp.clone()))
                    .collect::<Vec<_>>();
            }
        };

        if processing_options.security {
            chunks = chunks
                .into_iter()
                .map(|chunk| self.encrypt(chunk))
                .collect();
        };

        chunks
            .into_iter()
            .flat_map(|payload| self.to_frame(payload, self.frame_size()))
            .map(|frame| self.to_transport_packet(frame))
            .collect::<Vec<_>>()
    }
}

/// Dyn-compatible mirror of [`ProcessingPipeline`].
///
/// All associated constants from the sub-traits are exposed as methods so the
/// trait can be used as `dyn DynProcessingPipeline<Ts, Fr, Pkt>`, erasing the
/// concrete pipeline type while keeping `Ts`, `Fr`, and `Pkt`.
///
/// Implement [`ProcessingPipeline`] on your concrete type; the blanket impl
/// below provides `DynProcessingPipeline` for free.
pub trait DynProcessingPipeline<Ts, Fr, Pkt> {
    fn packet_size(&self) -> usize;

    // --- overhead accessors (mirrors of the supertrait associated constants) ---
    fn framing_overhead(&self) -> usize;
    fn transport_overhead(&self) -> usize;
    fn reliability_overhead(&self) -> usize;
    fn routing_overhead(&self) -> usize;
    fn nb_frames(&self) -> usize;

    // --- derived sizing helpers ---
    fn frame_size(&self) -> usize {
        self.packet_size() - self.transport_overhead() - self.framing_overhead()
    }

    fn chunk_size(&self, processing_options: StreamOptions) -> usize {
        let mut chunk_size = self.frame_size();
        if processing_options.security {
            chunk_size = chunk_size * self.nb_frames() - self.routing_overhead();
        }
        if processing_options.reliability {
            chunk_size -= self.reliability_overhead();
        }
        chunk_size
    }

    // --- buffer size from obfusctation ---
    fn obfusctaion_buffer_size(&self) -> usize;

    fn process(
        &mut self,
        input: Vec<u8>,
        processing_options: StreamOptions,
        timestamp: Ts,
    ) -> Vec<TimedData<Ts, Pkt>>;
}

impl<T, Ts, Fr, Pkt> DynProcessingPipeline<Ts, Fr, Pkt> for T
where
    T: ProcessingPipeline<Ts, Fr, Pkt>,
    Ts: Clone,
{
    fn packet_size(&self) -> usize {
        ProcessingPipeline::packet_size(self)
    }

    fn framing_overhead(&self) -> usize {
        <T as Framing<_, _>>::OVERHEAD_SIZE
    }

    fn transport_overhead(&self) -> usize {
        <T as Transport<_, _, _>>::OVERHEAD_SIZE
    }

    fn reliability_overhead(&self) -> usize {
        <T as Reliability<_>>::OVERHEAD_SIZE
    }

    fn routing_overhead(&self) -> usize {
        <T as RoutingSecurity<_>>::OVERHEAD_SIZE
    }

    fn obfusctaion_buffer_size(&self) -> usize {
        self.buffer_size()
    }

    fn nb_frames(&self) -> usize {
        self.nb_frames()
    }

    fn process(
        &mut self,
        input: Vec<u8>,
        processing_options: StreamOptions,
        timestamp: Ts,
    ) -> Vec<TimedData<Ts, Pkt>> {
        ProcessingPipeline::process(self, input, processing_options, timestamp)
    }
}

// SW How to integrate common::UnwrappingPipeline into that?
pub trait ClientUnwrappingPipeline<Ts, Pkt> {
    fn unwrap(&mut self, input: Pkt, timestamp: Ts) -> Option<Vec<u8>>;
}
