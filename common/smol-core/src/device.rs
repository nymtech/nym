// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

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
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub const DEFAULT_MTU: usize = 1500;

#[cfg(any(target_os = "android", target_os = "ios"))]
pub const DEFAULT_MTU: usize = 1280;

/// Client MTU fallback for IPRs that predate MTU negotiation (the v10 connect
/// response); at or below the historic 1420-byte IPR TUN.
pub const CLIENT_MTU_FALLBACK: usize = 1420;

/// Async adapter bridging mpsc channels to tokio-smoltcp's [`AsyncDevice`] trait.
///
/// Incoming packets (transport → smoltcp) arrive via the `rx` channel as a
/// [`Stream`]. Outgoing packets (smoltcp → transport) are sent via the `tx`
/// channel as a [`Sink`]. Both carry raw IP packets as `Vec<u8>`.
pub struct ChannelDevice {
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
    tx: mpsc::UnboundedSender<Vec<u8>>,
    capabilities: DeviceCapabilities,
    /// Latched once the inbound channel terminates; see [`Stream::poll_next`].
    rx_terminated: bool,
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
        negotiated_mtu: Option<usize>,
    ) -> Self {
        let mut capabilities = DeviceCapabilities::default();
        capabilities.medium = Medium::Ip;
        capabilities.max_transmission_unit = client_mtu(negotiated_mtu);
        // Leave `max_burst_size` unbounded (default `None`). Capping it at 1 made
        // smoltcp process a single packet per poll, serializing the datapath and
        // throttling bulk transfers; the channel transport imposes no burst limit.
        capabilities.max_burst_size = None;

        Self {
            rx,
            tx,
            capabilities,
            rx_terminated: false,
        }
    }
}

/// Client MTU precedence: `SMOLMIX_MTU` override >
/// min(Android's mobile MTU, the IPR-negotiated MTU (v10 connect response))
/// > a fallback for pre-v10 IPRs.
pub(crate) fn client_mtu(negotiated: Option<usize>) -> usize {
    if let Some(overrided) = std::env::var("SMOLMIX_MTU")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
    {
        return overrided;
    }

    if let Some(negotiated) = negotiated {
        return negotiated.min(DEFAULT_MTU);
    }

    CLIENT_MTU_FALLBACK
}

// tokio-smoltcp's reactor polls poll_next() to pull packets into the smoltcp
// Interface for processing.
impl Stream for ChannelDevice {
    type Item = io::Result<Vec<u8>>;

    /// Yields inbound packets, and **never terminates**: once the transport's sender is gone this
    /// parks forever rather than reporting end-of-stream.
    ///
    /// The reactor awaits this stream as one arm of a `select!` and discards the arm's result. A
    /// `Ready(None)` there is permanently ready, so the reactor would complete that arm on every
    /// iteration and spin without ever returning `Poll::Pending` - pegging a core and, because a
    /// tokio worker can only be reclaimed between polls, wedging runtime shutdown forever. Parking
    /// is safe: the reactor's other arms (poll-delay timer, socket notify, stopper) still drive it,
    /// and a dead transport has no further packets to deliver.
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.rx_terminated {
            return Poll::Pending;
        }
        match Pin::new(&mut self.rx).poll_next(cx) {
            Poll::Ready(None) => {
                self.rx_terminated = true;
                Poll::Pending
            }
            other => other.map(|opt| opt.map(Ok)),
        }
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
