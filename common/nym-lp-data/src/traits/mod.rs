// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Debug;
use std::sync::mpsc;

pub use helpers::{NoOpObfusctation, NoOpReliability, NoOpSecurity};

mod helpers;

pub struct TimedData<P, Ts> {
    pub data: P,
    pub timestamp: Ts,
}

impl<P, Ts> Debug for TimedData<P, Ts>
where
    P: Debug,
    Ts: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "TimedData {{")?;
        writeln!(f, "    data:")?;
        let data_debug = format!("{:#?}", &self.data);
        for line in data_debug.lines() {
            writeln!(f, "        {}", line)?;
        }
        writeln!(f, "    timestamp: {:#?},", &self.timestamp)?;
        write!(f, "}}")
    }
}

impl<P, Ts> TimedData<P, Ts> {
    pub fn data_transform<F>(self, mut op: F) -> Self
    where
        F: FnMut(P) -> P,
    {
        TimedData {
            data: op(self.data),
            timestamp: self.timestamp,
        }
    }

    pub fn ts_transform<F>(self, mut op: F) -> Self
    where
        F: FnMut(Ts) -> Ts,
    {
        TimedData {
            data: self.data,
            timestamp: op(self.timestamp),
        }
    }
}

/// Helper type to erase the Vec<u8> parameters
pub type TimedPayload<Ts> = TimedData<Vec<u8>, Ts>;

#[derive(Clone, Copy, Debug)]
pub struct StreamOptions {
    pub reliability: bool,
    pub security: bool,
    pub obfuscation: bool,
}

impl Default for StreamOptions {
    fn default() -> Self {
        Self {
            reliability: true,
            security: true,
            obfuscation: true,
        }
    }
}

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
/// - A `Vec<TimedData<Fr, Ts>>`, result of the framing operation.
pub trait Framing<Ts, Fr> {
    const OVERHEAD_SIZE: usize;
    fn to_frame(&self, payload: TimedPayload<Ts>, frame_size: usize) -> Vec<TimedData<Fr, Ts>>;
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
/// - A `TimedData<P, Ts>`, result of the transport operation.
pub trait Transport<Ts, Fr, P> {
    const OVERHEAD_SIZE: usize;
    fn to_transport_packet(&self, frame: TimedData<Fr, Ts>) -> TimedData<P, Ts>;
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
pub trait ProcessingPipeline<Ts, Fr, P>:
    Chunking<Ts>
    + Reliability<Ts>
    + Security<Ts>
    + Obfuscation<Ts>
    + Framing<Ts, Fr>
    + Transport<Ts, Fr, P>
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
    ) -> Vec<TimedData<P, Ts>> {
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

        if processing_options.security {
            chunks = chunks
                .into_iter()
                .map(|chunk| self.encrypt(chunk))
                .collect();
        };

        if processing_options.obfuscation {
            chunks = chunks
                .into_iter()
                .flat_map(|chunk| self.obfuscate(chunk, timestamp.clone()))
                .collect::<Vec<_>>();
        };

        chunks
            .into_iter()
            .flat_map(|payload| self.to_frame(payload, self.frame_size()))
            .map(|frame| self.to_transport_packet(frame))
            .collect::<Vec<_>>()
    }
}

/// The generic pipeline struct
pub struct Pipeline<C, R, S, O, F, T> {
    pub packet_size: usize,
    pub chunking: C,
    pub reliability: R,
    pub security: S,
    pub obfuscation: O,
    pub framing: F,
    pub transport: T,
}

impl<Ts, C, R, S, O, F, T> Chunking<Ts> for Pipeline<C, R, S, O, F, T>
where
    C: Chunking<Ts>,
{
    fn chunked(&self, input: Vec<u8>, chunk_size: usize, timestamp: Ts) -> Vec<TimedPayload<Ts>> {
        self.chunking.chunked(input, chunk_size, timestamp)
    }
}

impl<Ts, C, R, S, O, F, T> Reliability<Ts> for Pipeline<C, R, S, O, F, T>
where
    R: Reliability<Ts>,
{
    const OVERHEAD_SIZE: usize = R::OVERHEAD_SIZE;

    fn reliable_encode(&self, input: TimedPayload<Ts>) -> TimedPayload<Ts> {
        self.reliability.reliable_encode(input)
    }
}

impl<Ts, C, R, S, O, F, T> Security<Ts> for Pipeline<C, R, S, O, F, T>
where
    S: Security<Ts>,
{
    const OVERHEAD_SIZE: usize = S::OVERHEAD_SIZE;
    fn nb_frames(&self) -> usize {
        self.security.nb_frames()
    }

    fn encrypt(&self, input: TimedPayload<Ts>) -> TimedPayload<Ts> {
        self.security.encrypt(input)
    }
}

impl<Ts, C, R, S, O, F, T> Obfuscation<Ts> for Pipeline<C, R, S, O, F, T>
where
    O: Obfuscation<Ts>,
{
    fn obfuscate(&mut self, input: TimedPayload<Ts>, timestamp: Ts) -> Vec<TimedPayload<Ts>> {
        self.obfuscation.obfuscate(input, timestamp)
    }
    fn buffer_size(&self) -> usize {
        self.obfuscation.buffer_size()
    }
}

impl<Ts, C, R, S, O, F, T, Fr> Framing<Ts, Fr> for Pipeline<C, R, S, O, F, T>
where
    F: Framing<Ts, Fr>,
{
    const OVERHEAD_SIZE: usize = F::OVERHEAD_SIZE;

    fn to_frame(&self, payload: TimedPayload<Ts>, frame_size: usize) -> Vec<TimedData<Fr, Ts>> {
        self.framing.to_frame(payload, frame_size)
    }
}

impl<Ts, C, R, S, O, F, T, Fr, P> Transport<Ts, Fr, P> for Pipeline<C, R, S, O, F, T>
where
    T: Transport<Ts, Fr, P>,
{
    const OVERHEAD_SIZE: usize = T::OVERHEAD_SIZE;

    fn to_transport_packet(&self, frame: TimedData<Fr, Ts>) -> TimedData<P, Ts> {
        self.transport.to_transport_packet(frame)
    }
}

impl<Ts, C, R, S, O, F, T, Fr, P> ProcessingPipeline<Ts, Fr, P> for Pipeline<C, R, S, O, F, T>
where
    Ts: Clone,
    C: Chunking<Ts>,
    R: Reliability<Ts>,
    S: Security<Ts>,
    O: Obfuscation<Ts>,
    F: Framing<Ts, Fr>,
    T: Transport<Ts, Fr, P>,
{
    fn packet_size(&self) -> usize {
        self.packet_size
    }
}

pub struct PipelineDriver<Ts, Fr, P, Pl>
where
    Pl: ProcessingPipeline<Ts, Fr, P>,
    Ts: Clone + PartialOrd,
{
    pipeline: Pl,
    processing_options: StreamOptions,

    packet_buffer: Vec<TimedData<P, Ts>>,

    input: mpsc::Receiver<Vec<u8>>,

    // Keeping a ref so we don't have problem about it being dropped
    input_sender: mpsc::SyncSender<Vec<u8>>,
    _marker: std::marker::PhantomData<Fr>,
}

impl<Ts, Fr, P, Pl> PipelineDriver<Ts, Fr, P, Pl>
where
    Pl: ProcessingPipeline<Ts, Fr, P>,
    Ts: Clone + PartialOrd,
{
    pub fn new(pipeline: Pl) -> Self {
        let (input_sender, input_receiver) = mpsc::sync_channel(0);

        Self {
            pipeline,
            processing_options: Default::default(),
            packet_buffer: Vec::new(),
            input: input_receiver,
            input_sender,
            _marker: std::marker::PhantomData,
        }
    }

    #[must_use]
    pub fn with_processing_options(mut self, processing_options: StreamOptions) -> Self {
        self.processing_options = processing_options;
        self
    }

    pub fn input_sender(&self) -> mpsc::SyncSender<Vec<u8>> {
        self.input_sender.clone()
    }

    pub fn tick(&mut self, timestamp: Ts) -> Vec<P> {
        // We're reading a message only if
        // - a: Our buffer is empty
        // - b: Obfuscation layer reports an empty buffer
        // Otherwise, we will have buffers adding latencies to data
        let next_message = if self.packet_buffer.is_empty() && self.pipeline.buffer_size() == 0 {
            self.input.try_recv().unwrap_or_else(|_| {
                tracing::trace!("No message in the queue");
                Vec::new()
            })
        } else {
            Vec::new()
        };
        self.packet_buffer.extend(self.pipeline.process(
            next_message,
            self.processing_options,
            timestamp.clone(),
        ));

        self.packet_buffer
            .extract_if(.., |p| p.timestamp <= timestamp)
            .map(|pkt| pkt.data)
            .collect()
    }
}
