// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;
use std::sync::mpsc;
use std::time::Instant;

use crate::AddressedTimedData;
use crate::clients::traits::DynClientWrappingPipeline;

/// Drives a [`DynClientWrappingPipeline`] tick-by-tick, feeding it raw application
/// payloads and emitting transport packets whose scheduled timestamp is due.
///
/// ## How it works
///
/// 1. The caller submits raw byte payloads via [`ClientWrappingPipelineDriver::input_sender`].
/// 2. On each call to [`ClientWrappingPipelineDriver::tick`], the driver reads one pending
///    payload (only when both the packet buffer and the obfuscation buffer are
///    empty, to avoid adding extra latency on top of buffered data), runs it
///    through the pipeline, and appends the resulting timestamped packets to an
///    internal buffer.
/// 3. Packets whose `timestamp ≤ now` are extracted from the buffer and
///    returned to the caller for sending.
///
/// Timestamps are [`Instant`]s, compared with `≤` to decide which packets are due.
///
pub struct ClientWrappingPipelineDriver<Pkt, Opts> {
    pipeline: Box<dyn DynClientWrappingPipeline<Pkt, Opts>>,

    packet_buffer: Vec<AddressedTimedData<Pkt>>,

    input: mpsc::Receiver<(Vec<u8>, Opts, SocketAddr)>,

    // Keeping a ref so we don't have problem about it being dropped
    input_sender: mpsc::SyncSender<(Vec<u8>, Opts, SocketAddr)>,
}

impl<Pkt, Opts> ClientWrappingPipelineDriver<Pkt, Opts> {
    /// Create a new driver wrapping `pipeline`.
    ///
    /// Internally allocates a zero-capacity `sync_channel` for input payloads.
    pub fn new(pipeline: impl DynClientWrappingPipeline<Pkt, Opts> + 'static) -> Self {
        let (input_sender, input_receiver) = mpsc::sync_channel(0);

        Self {
            pipeline: Box::new(pipeline),
            packet_buffer: Vec::new(),
            input: input_receiver,
            input_sender,
        }
    }

    /// Return a clone of the sender half of the input channel.
    ///
    /// Send raw application payloads here; they will be picked up on the next
    /// tick when the pipeline's internal buffers are empty.
    pub fn input_sender(&self) -> mpsc::SyncSender<(Vec<u8>, Opts, SocketAddr)> {
        self.input_sender.clone()
    }

    /// Advance the driver by one tick.
    ///
    /// Reads a pending input payload (if both the packet buffer and the
    /// obfuscation buffer are empty), runs it through the pipeline, then
    /// returns all packets whose `timestamp ≤ now`.
    pub fn tick(&mut self, timestamp: Instant) -> Vec<(Pkt, SocketAddr)> {
        // We're reading a message only if our buffer is empty
        // Otherwise, we will have buffers adding latencies to data
        let next_message = if self.packet_buffer.is_empty() {
            self.input
                .try_recv()
                .inspect_err(|_| tracing::trace!("No message in the queue"))
                .ok()
        } else {
            None
        };
        match self.pipeline.process(next_message, timestamp) {
            Ok(packets) => self.packet_buffer.extend(packets),
            Err(e) => tracing::error!("Failed to wrap outgoing packets : {e}"),
        }

        self.packet_buffer
            .extract_if(.., |p| p.data.timestamp <= timestamp)
            .map(|pkt| (pkt.data.data, pkt.dst))
            .collect()
    }
}
