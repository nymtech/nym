// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

//! Async device adapter for tokio-smoltcp.
//!
//! Wraps mpsc channel ends (connected to [`NymIprBridge`](crate::bridge::NymIprBridge))
//! in the [`Stream`]/[`Sink`] traits that tokio-smoltcp requires. See the
//! [crate-level docs](crate) for how this fits into the full stack.

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::channel::mpsc;
use futures::{Sink, Stream};
use smoltcp::phy::{DeviceCapabilities, Medium};
use tokio_smoltcp::device::AsyncDevice;

/// Async adapter bridging mpsc channels to tokio-smoltcp's [`AsyncDevice`] trait.
///
/// Incoming packets (mixnet → smoltcp) arrive via the `rx` channel as a [`Stream`].
/// Outgoing packets (smoltcp → mixnet) are sent via the `tx` channel as a [`Sink`].
pub(crate) struct NymAsyncDevice {
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
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

// tokio-smoltcp calls poll_next() in its reactor loop to feed packets into the
// smoltcp Interface for processing.
impl Stream for NymAsyncDevice {
    type Item = io::Result<Vec<u8>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx).poll_next(cx).map(|opt| opt.map(Ok))
    }
}

// When smoltcp produces a packet (e.g. TCP SYN, data segment, UDP datagram),
// tokio-smoltcp sends it here and we forward it to the bridge for mixnet delivery.
//
// Delegates to the built-in Sink impl on futures::channel::mpsc::UnboundedSender,
// which handles channel liveness checks (poll_ready) and disconnect (poll_close).
impl Sink<Vec<u8>> for NymAsyncDevice {
    type Error = io::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.tx)
            .poll_ready(cx)
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "bridge channel closed"))
    }

    fn start_send(mut self: Pin<&mut Self>, item: Vec<u8>) -> Result<(), Self::Error> {
        Pin::new(&mut self.tx)
            .start_send(item)
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "bridge channel closed"))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.tx)
            .poll_flush(cx)
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "bridge channel closed"))
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.tx)
            .poll_close(cx)
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "bridge channel closed"))
    }
}

impl AsyncDevice for NymAsyncDevice {
    fn capabilities(&self) -> &DeviceCapabilities {
        &self.capabilities
    }
}
