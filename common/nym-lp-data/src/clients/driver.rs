// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// use std::sync::mpsc;

// use crate::TimedData;
// use crate::clients::traits::DynClientWrappingPipeline;
// use crate::clients::types::StreamOptions;

// SW same here, adapt if needed

// /// Drives a [`DynClientWrappingPipeline`] tick-by-tick, feeding it raw application
// /// payloads and emitting transport packets whose scheduled timestamp is due.
// ///
// /// ## How it works
// ///
// /// 1. The caller submits raw byte payloads via [`PipelineDriver::input_sender`].
// /// 2. On each call to [`PipelineDriver::tick`], the driver reads one pending
// ///    payload (only when both the packet buffer and the obfuscation buffer are
// ///    empty, to avoid adding extra latency on top of buffered data), runs it
// ///    through the pipeline, and appends the resulting timestamped packets to an
// ///    internal buffer.
// /// 3. Packets whose `timestamp ≤ now` are extracted from the buffer and
// ///    returned to the caller for sending.
// ///
// /// `Ts` must implement `Clone + PartialOrd` so that timestamps can be compared
// /// to decide which packets are due.
// pub struct ClientWrappingPipelineDriver<Ts, Fr, Pkt, Opts, I>
// where
//     Ts: Clone + PartialOrd,
//     I: LpInput<Opts>,
// {
//     pipeline: Box<dyn DynClientWrappingPipeline<Ts, Fr, Pkt, Opts, I>>,
//     processing_options: StreamOptions,

//     packet_buffer: Vec<TimedData<Ts, Pkt>>,

//     input: mpsc::Receiver<I>,

//     // Keeping a ref so we don't have problem about it being dropped
//     input_sender: mpsc::SyncSender<I>,
//     _marker: std::marker::PhantomData<Fr>,
// }

// impl<Ts, Fr, Pkt, Opts, I> ClientWrappingPipelineDriver<Ts, Fr, Pkt, Opts, I>
// where
//     Ts: Clone + PartialOrd,
//     I: LpInput<Opts>,
// {
//     /// Create a new driver wrapping `pipeline`.
//     ///
//     /// Internally allocates a zero-capacity `sync_channel` for input payloads.
//     /// All three optional pipeline stages are enabled by default
//     /// ([`StreamOptions::default`]); use [`with_processing_options`] to
//     /// override.
//     ///
//     /// [`with_processing_options`]: PipelineDriver::with_processing_options
//     pub fn new(pipeline: impl DynClientWrappingPipeline<Ts, Fr, Pkt, Opts, I> + 'static) -> Self {
//         let (input_sender, input_receiver) = mpsc::sync_channel(0);

//         Self {
//             pipeline: Box::new(pipeline),
//             processing_options: Default::default(),
//             packet_buffer: Vec::new(),
//             input: input_receiver,
//             input_sender,
//             _marker: std::marker::PhantomData,
//         }
//     }

//     /// Override the [`StreamOptions`] used when processing payloads.
//     #[must_use]
//     pub fn with_processing_options(mut self, processing_options: StreamOptions) -> Self {
//         self.processing_options = processing_options;
//         self
//     }

//     /// Return a clone of the sender half of the input channel.
//     ///
//     /// Send raw application payloads here; they will be picked up on the next
//     /// tick when the pipeline's internal buffers are empty.
//     pub fn input_sender(&self) -> mpsc::SyncSender<I> {
//         self.input_sender.clone()
//     }

//     /// Advance the driver by one tick.
//     ///
//     /// Reads a pending input payload (if both the packet buffer and the
//     /// obfuscation buffer are empty), runs it through the pipeline, then
//     /// returns all packets whose `timestamp ≤ now`.
//     pub fn tick(&mut self, timestamp: Ts) -> Vec<Pkt> {
//         // We're reading a message only if
//         // - a: Our buffer is empty
//         // - b: Obfuscation layer reports an empty buffer
//         // Otherwise, we will have buffers adding latencies to data
//         let next_message =
//             if self.packet_buffer.is_empty() && self.pipeline.obfusctaion_buffer_size() == 0 {
//                 self.input
//                     .try_recv()
//                     .inspect_err(|_| tracing::trace!("No message in the queue"))
//                     .ok()
//             } else {
//                 None
//             };
//         self.packet_buffer.extend(self.pipeline.process(
//             next_message,
//             self.processing_options,
//             timestamp.clone(),
//         ));

//         self.packet_buffer
//             .extract_if(.., |p| p.timestamp <= timestamp)
//             .map(|pkt| pkt.data)
//             .collect()
//     }
// }
