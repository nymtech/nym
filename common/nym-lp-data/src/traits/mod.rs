// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use helpers::{NoOpObfusctation, NoOpReliability, NoOpSecurity};

use crate::traits::types::{StreamOptions, TimedData, TimedPayload};

mod helpers;
pub mod types;

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
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the `TimedPayload`.
pub trait Obfuscation<Ts> {
    /// Obfuscate a given timed payload
    /// # Parameters
    /// - `input`: Payload to encode with the encryption mechanism.
    /// - `timestamp` : Current timestamp
    ///
    /// # Returns
    /// - An `Vec<TimedPayload>`, result of the obfuscation algorithm
    /// - The vector can be empty if there is nothing to return right away
    fn obfuscate(&mut self, input: TimedPayload<Ts>, timestamp: Ts) -> Vec<TimedPayload<Ts>>;

    /// Return the size of the inner timed payload buffer, to help with backpressure
    fn buffer_size(&self) -> usize;
}

/// Trait for applying encryption to a timed payload.
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the `TimedPayload`.
///
/// # Associated Constants
/// - `OVERHEAD_SIZE`: Number of additional bytes added by the encryption scheme.
/// - `nb_frames`: Number of frames used by an encrypted payload (default is 1)
///
/// # Parameters
/// - `input`: Payload to encode with the encryption mechanism.
///
/// # Returns
/// - A `TimedPayload` containing the encrypted data.
pub trait Security<Ts> {
    const OVERHEAD_SIZE: usize;
    fn nb_frames(&self) -> usize {
        1
    }
    fn encrypt(&self, input: TimedPayload<Ts>) -> TimedPayload<Ts>;
}

/// Trait for applying framing to a timed payload.
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the `TimedPayload`.
/// - `Fr`: Frame type that will be returned.
///
/// # Associated Constants
/// - `OVERHEAD_SIZE`: Number of additional bytes added by the framing scheme.
///
/// # Parameters
/// - `payload`: Payload frame.
/// - `framesize` : The size of the frame.
///
/// # Returns
/// - A `Vec<TimedData<Ts, Fr>>`, result of the framing operation.
pub trait Framing<Ts, Fr> {
    const OVERHEAD_SIZE: usize;
    fn to_frame(&self, payload: TimedPayload<Ts>, frame_size: usize) -> Vec<TimedData<Ts, Fr>>;
}

/// Trait for applying tranport layer to a timed payload.
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the `TimedPayload`.
/// - `Fr`: Frame type used in input.
/// - `P` : Packet type to return.
///
/// # Associated Constants
/// - `OVERHEAD_SIZE`: Number of additional bytes added by the transport scheme.
///
/// # Parameters
/// - `frame`: Input Frame.
///
/// # Returns
/// - A `TimedData<Ts, Pkt>`, result of the transport operation.
pub trait Transport<Ts, Fr, Pkt> {
    const OVERHEAD_SIZE: usize;
    fn to_transport_packet(&self, frame: TimedData<Ts, Fr>) -> TimedData<Ts, Pkt>;
}

/// Trait for a message pipeline.
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the `TimedPayload`.
/// - `Fr`: Frame type used in input.
/// - `P` : Packet type to return.
///
/// # Associated Constants
/// - `packet_size`: Size of the outputted packets.
pub trait ProcessingPipeline<Ts, Fr, Pkt>:
    Chunking<Ts>
    + Reliability<Ts>
    + Obfuscation<Ts>
    + Security<Ts>
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
            chunk_size = chunk_size * self.nb_frames() - <Self as Security<_>>::OVERHEAD_SIZE;
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
            chunks = chunks
                .into_iter()
                .flat_map(|chunk| self.obfuscate(chunk, timestamp.clone()))
                .collect::<Vec<_>>();
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
