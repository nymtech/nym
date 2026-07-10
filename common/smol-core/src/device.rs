// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

//! Async device adapter for tokio-smoltcp.
//!
//! [`ChannelDevice`] wraps a pair of mpsc channel ends (the abstract IP-packet
//! transport) in the [`Stream`]/[`Sink`] traits that tokio-smoltcp's
//! [`AsyncDevice`] requires. It is fully transport-agnostic: anything that can
//! produce inbound IP packets (`Vec<u8>`) and consume outbound ones — a mixnet
//! bridge, a WireGuard datapath, a loopback test harness — drives the same
//! stack. This is the seam extracted from `smolmix`'s `NymAsyncDevice`.

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::channel::mpsc;
use futures::{Sink, Stream};
use smoltcp::phy::{DeviceCapabilities, Medium};
use tokio_smoltcp::device::AsyncDevice;

/// Default MTU for the virtual interface if none is specified.
pub const DEFAULT_MTU: usize = 1500;

/// Async adapter bridging mpsc channels to tokio-smoltcp's [`AsyncDevice`] trait.
///
/// Incoming packets (transport → smoltcp) arrive via the `rx` channel as a
/// [`Stream`]. Outgoing packets (smoltcp → transport) are sent via the `tx`
/// channel as a [`Sink`]. Both carry raw IP packets as `Vec<u8>`.
pub struct ChannelDevice {
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
    tx: mpsc::UnboundedSender<Vec<u8>>,
    capabilities: DeviceCapabilities,
}

impl ChannelDevice {
    /// Build a device from the transport's channel ends and an MTU.
    ///
    /// - `rx`: inbound IP packets (transport → smoltcp)
    /// - `tx`: outbound IP packets (smoltcp → transport)
    /// - `mtu`: max transmission unit for the virtual interface
    pub fn new(
        rx: mpsc::UnboundedReceiver<Vec<u8>>,
        tx: mpsc::UnboundedSender<Vec<u8>>,
        mtu: usize,
    ) -> Self {
        let mut capabilities = DeviceCapabilities::default();
        capabilities.medium = Medium::Ip;
        capabilities.max_transmission_unit = mtu;
        // Leave `max_burst_size` unbounded (default `None`). Capping it at 1 made
        // smoltcp process a single packet per poll, serializing the datapath and
        // throttling bulk transfers; the channel transport imposes no burst limit.
        capabilities.max_burst_size = None;

        Self {
            rx,
            tx,
            capabilities,
        }
    }
}

// tokio-smoltcp's reactor polls poll_next() to pull packets into the smoltcp
// Interface for processing.
impl Stream for ChannelDevice {
    type Item = io::Result<Vec<u8>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx).poll_next(cx).map(|opt| opt.map(Ok))
    }
}

// When smoltcp produces a packet, tokio-smoltcp hands it to this Sink, which
// forwards it to the transport. Delegates to the mpsc UnboundedSender Sink impl,
// which already handles liveness (poll_ready) and disconnect (poll_close).
impl Sink<Vec<u8>> for ChannelDevice {
    type Error = io::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.tx)
            .poll_ready(cx)
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "transport channel closed"))
    }

    fn start_send(mut self: Pin<&mut Self>, item: Vec<u8>) -> Result<(), Self::Error> {
        Pin::new(&mut self.tx)
            .start_send(item)
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "transport channel closed"))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.tx)
            .poll_flush(cx)
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "transport channel closed"))
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.tx)
            .poll_close(cx)
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "transport channel closed"))
    }
}

impl AsyncDevice for ChannelDevice {
    fn capabilities(&self) -> &DeviceCapabilities {
        &self.capabilities
    }
}
