// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

use smoltcp::{
    phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken},
    time::Instant,
};
use std::collections::VecDeque;
use tokio::sync::mpsc;
use tracing::{debug, info, trace, warn};

/// # Overview
/// We need something to bridge the async / sync weirdness (Device trait fns are sync, IpMixStream fns are
/// async) in a way that allows for the `NymIprDevice` to look and act like any other device.
///
/// cf smoltcp's loopback.rs:
/// ```
/// pub struct Loopback {
///     queue: VecDeque<Vec<u8>>,
/// }
///
/// impl Device for Loopback {
///     type RxToken<'a> = RxToken;
///     type TxToken<'a> = TxToken<'a>;
///
///     fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
///         self.queue.pop_front().map(move |buffer| {
///             let rx = RxToken { buffer };
///             let tx = TxToken {
///                 queue: &mut self.queue,
///             };
///             (rx, tx)
///         })
///     }
/// }
/// ```
///
/// We need to be polling the queue to/from the NymIprBridge, hence the addition of the
/// mpsc channels in the Device struct and the extra fns.
///
/// # Architecture
///
/// smoltcp (sync) <-> NymIprDevice <-> channels <-> NymIprBridge <-> Mixnet (async)
///
/// The device maintains a receive queue for packets coming from the mixnet and
/// uses unbounded channels to communicate with the bridge task that handles the
/// actual mixnet I/O. We poll the channel in receive() to move packets via mpsc
/// from async to sync world.
///
/// This way no blocking from smoltcp + allows for concurrency.
///
/// Adapter pattern between sync polling-based I/O and async event-based I/O.
pub struct NymIprDevice {
    // Receive queue for packets coming from the mixnet
    rx_queue: VecDeque<Vec<u8>>,

    // Channel to send packets to the bridge task
    tx_sender: mpsc::UnboundedSender<Vec<u8>>,

    // Device capabilities
    capabilities: DeviceCapabilities,

    // Channel to receive packets from the bridge task
    rx_receiver: mpsc::UnboundedReceiver<Vec<u8>>,
}

impl NymIprDevice {
    pub fn new(
        tx_sender: mpsc::UnboundedSender<Vec<u8>>,
        rx_receiver: mpsc::UnboundedReceiver<Vec<u8>>,
    ) -> Self {
        let mut capabilities = DeviceCapabilities::default();
        capabilities.medium = Medium::Ip;
        // Standard MTU for IP packets - TODO make configurable
        capabilities.max_transmission_unit = 1500;
        // Process one packet at a time. TODO experiment with this
        capabilities.max_burst_size = Some(1);

        Self {
            rx_queue: VecDeque::new(),
            tx_sender,
            capabilities,
            rx_receiver,
        }
    }

    /// Poll for new packets from the bridge
    fn poll_rx_queue(&mut self) {
        // Try to receive all available packets without blocking, queue them for smoltcp consumption.
        while let Ok(packet) = self.rx_receiver.try_recv() {
            trace!("Received packet of {} bytes from bridge", packet.len());
            self.rx_queue.push_back(packet);
        }
    }
}

impl Device for NymIprDevice {
    type RxToken<'a>
        = NymRxToken
    where
        Self: 'a;
    type TxToken<'a>
        = NymTxToken
    where
        Self: 'a;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        // Poll for new packets from the async bridge
        self.poll_rx_queue();

        // Check if we have a packet to deliver
        let packet = self.rx_queue.pop_front()?;

        // Create tokens - RxToken owns the packet data
        let rx_token = NymRxToken { buffer: packet };
        let tx_token = NymTxToken {
            tx_sender: self.tx_sender.clone(),
        };

        Some((rx_token, tx_token))
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        // We can always transmit (channel will buffer)
        Some(NymTxToken {
            tx_sender: self.tx_sender.clone(),
        })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        self.capabilities.clone()
    }
}

/// Receive token - owns the packet buffer
pub struct NymRxToken {
    buffer: Vec<u8>,
}

impl RxToken for NymRxToken {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&[u8]) -> R,
    {
        debug!("Consuming RX packet of {} bytes", self.buffer.len());
        f(&self.buffer)
    }
}

/// Transmit token - holds channel sender
pub struct NymTxToken {
    tx_sender: mpsc::UnboundedSender<Vec<u8>>,
}

impl TxToken for NymTxToken {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        // Create buffer for the packet
        let mut buffer = vec![0u8; len];

        // Let smoltcp fill the packet
        let result = f(&mut buffer);

        // Send raw packet to the bridge task for transmission
        if let Err(e) = self.tx_sender.send(buffer) {
            warn!("Failed to send packet to bridge: {}", e);
        } else {
            info!("Sent {} byte packet to bridge", len);
        }

        result
    }
}
