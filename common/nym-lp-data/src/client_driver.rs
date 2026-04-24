// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::mpsc;

use crate::traits::{ProcessingPipeline, types::StreamOptions, types::TimedData};

pub struct PipelineDriver<Ts, Fr, Pkt, Pl>
where
    Pl: ProcessingPipeline<Ts, Fr, Pkt>,
    Ts: Clone + PartialOrd,
{
    pipeline: Pl,
    processing_options: StreamOptions,

    packet_buffer: Vec<TimedData<Ts, Pkt>>,

    input: mpsc::Receiver<Vec<u8>>,

    // Keeping a ref so we don't have problem about it being dropped
    input_sender: mpsc::SyncSender<Vec<u8>>,
    _marker: std::marker::PhantomData<Fr>,
}

impl<Ts, Fr, Pkt, Pl> PipelineDriver<Ts, Fr, Pkt, Pl>
where
    Pl: ProcessingPipeline<Ts, Fr, Pkt>,
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

    pub fn tick(&mut self, timestamp: Ts) -> Vec<Pkt> {
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
