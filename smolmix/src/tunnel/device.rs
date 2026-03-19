// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

//! Async device adapter for tokio-smoltcp.
//!
//! tokio-smoltcp expects an [`AsyncDevice`] — something that is both a [`Stream`] of incoming
//! raw IP packets and a [`Sink`] for outgoing ones. It uses this to drive the smoltcp
//! `Interface` poll loop internally (retransmits, keepalives, TCP state machine, etc.).
//!
//! Our packets come from the Nym mixnet via [`NymIprBridge`](crate::NymIprBridge), which
//! already communicates over mpsc channels. So this adapter is thin: it just wraps those
//! channel ends in the `Stream`/`Sink` traits that tokio-smoltcp requires.
//!
//! ```text
//! mixnet ← IpMixStream ← NymIprBridge ← outgoing_tx ← Sink  ← smoltcp (via tokio-smoltcp)
//! mixnet → IpMixStream → NymIprBridge → incoming_rx → Stream → smoltcp (via tokio-smoltcp)
//! ```
//!
//! Medium::Ip means no Ethernet framing — raw IP packets go in and out, which matches
//! what the IPR protocol expects.

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{Sink, Stream};
use smoltcp::phy::{DeviceCapabilities, Medium};
use tokio::sync::mpsc;
use tokio_smoltcp::device::AsyncDevice;

/// Async adapter bridging mpsc channels (connected to [`NymIprBridge`](crate::NymIprBridge))
/// to tokio-smoltcp's [`AsyncDevice`] trait.
///
/// Incoming packets (mixnet → smoltcp) arrive via the `rx` channel as a [`Stream`].
/// Outgoing packets (smoltcp → mixnet) are sent via the `tx` channel as a [`Sink`].
pub(crate) struct NymAsyncDevice {
    /// Receives raw IP packets from the bridge (originally from mixnet/IPR).
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
    /// Sends raw IP packets to the bridge (onwards to mixnet/IPR).
    tx: mpsc::UnboundedSender<Vec<u8>>,
    capabilities: DeviceCapabilities,
}

impl NymAsyncDevice {
    pub(crate) fn new(
        rx: mpsc::UnboundedReceiver<Vec<u8>>,
        tx: mpsc::UnboundedSender<Vec<u8>>,
    ) -> Self {
        let mut capabilities = DeviceCapabilities::default();
        capabilities.medium = Medium::Ip;
        capabilities.max_transmission_unit = 1500;
        capabilities.max_burst_size = Some(1);

        Self {
            rx,
            tx,
            capabilities,
        }
    }
}

// Stream yields incoming IP packets from the bridge. tokio-smoltcp calls poll_next()
// in its reactor loop to feed packets into the smoltcp Interface for processing.
impl Stream for NymAsyncDevice {
    type Item = io::Result<Vec<u8>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // poll_recv returns Poll<Option<T>>; wrap the inner value in Ok since
        // our channel is infallible (errors only happen at the bridge level).
        self.rx.poll_recv(cx).map(|opt| opt.map(Ok))
    }
}

// Sink accepts outgoing IP packets from smoltcp. When smoltcp produces a packet
// (e.g. a TCP SYN, data segment, or UDP datagram), tokio-smoltcp sends it here,
// and we forward it to the bridge which bundles it for the mixnet.
//
// All Sink methods are trivial because the underlying mpsc channel is unbounded —
// it's always ready, never needs flushing, and never blocks. The real flow control
// happens at the mixnet layer (the bridge rate-limits via the IPR protocol).
impl Sink<Vec<u8>> for NymAsyncDevice {
    type Error = io::Error;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Vec<u8>) -> Result<(), Self::Error> {
        self.tx
            .send(item)
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "bridge channel closed"))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

impl AsyncDevice for NymAsyncDevice {
    fn capabilities(&self) -> &DeviceCapabilities {
        &self.capabilities
    }
}
