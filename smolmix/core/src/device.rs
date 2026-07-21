// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

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
        // Default client MTU. The IPR egress TUN is 1420 bytes
        // (common/tun/src/linux/tun_device.rs), so a 1500-byte client black-holes
        // large downloads: full-size segments are dropped with ICMP frag-needed at
        // the TUN, and there is no MSS clamp or MTU negotiation to warn the client.
        // Match nym-vpn-client: 1280 on Android to align with mobile, 1420 elsewhere.
        // SMOLMIX_MTU overrides for testing.
        #[cfg(target_os = "android")]
        const DEFAULT_MTU: usize = 1280;
        #[cfg(not(target_os = "android"))]
        const DEFAULT_MTU: usize = 1420;
        capabilities.max_transmission_unit = std::env::var("SMOLMIX_MTU")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_MTU);
        // Packets smoltcp drains per poll cycle. SMOLMIX_BURST overrides it (the
        // default of 1 means one inbound packet per full poll, which throttles
        // throughput -- this lets us test batching).
        let burst = std::env::var("SMOLMIX_BURST")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        capabilities.max_burst_size = Some(burst);

        Self {
            rx,
            tx,
            capabilities,
        }
    }
}

// tokio-smoltcp's reactor polls poll_next() to pull packets into the smoltcp
// Interface for processing.
impl Stream for NymAsyncDevice {
    type Item = io::Result<Vec<u8>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx).poll_next(cx).map(|opt| {
            opt.map(|pkt| {
                log_ipv4("IN", &pkt);
                Ok(pkt)
            })
        })
    }
}

// Env-gated IPv4 packet inspector (SMOLMIX_PKTLOG=1). Prints total length and
// fragmentation state so we can see whether the IPR fragments >1420-byte packets
// and whether those fragments reach the client (smoltcp drops them without the
// proto-ipv4-fragmentation feature).
fn log_ipv4(dir: &str, pkt: &[u8]) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static ON: AtomicBool = AtomicBool::new(false);
    static CHECKED: AtomicBool = AtomicBool::new(false);
    if !CHECKED.swap(true, Ordering::Relaxed) {
        ON.store(std::env::var("SMOLMIX_PKTLOG").is_ok(), Ordering::Relaxed);
    }
    if !ON.load(Ordering::Relaxed) || pkt.len() < 20 || (pkt[0] >> 4) != 4 {
        return;
    }
    let total = u16::from_be_bytes([pkt[2], pkt[3]]);
    let flags_frag = u16::from_be_bytes([pkt[6], pkt[7]]);
    let df = (flags_frag & 0x4000) != 0;
    let mf = (flags_frag & 0x2000) != 0;
    let off = (flags_frag & 0x1fff) * 8;
    let proto = pkt[9];
    // Decode the TCP header (seq/ack/flags/window) so we can tell retransmission
    // (inbound loss) from a zero-window reader stall.
    let ihl = ((pkt[0] & 0x0f) as usize) * 4;
    let mut tcp = String::new();
    if proto == 6 && pkt.len() >= ihl + 20 {
        let t = &pkt[ihl..];
        let seq = u32::from_be_bytes([t[4], t[5], t[6], t[7]]);
        let ack = u32::from_be_bytes([t[8], t[9], t[10], t[11]]);
        let flags = t[13];
        let win = u16::from_be_bytes([t[14], t[15]]);
        let doff = ((t[12] >> 4) as usize) * 4;
        let payload = total as usize - ihl - doff;
        // On a SYN, walk the TCP options to extract the advertised MSS (kind 2).
        // This is what invites the peer's segment size; MSS 1460 at client MTU 1500
        // means the server may send 1500-byte packets that overflow the IPR TUN.
        let mut mss = String::new();
        if flags & 0x02 != 0 && t.len() >= doff {
            let opts = &t[20..doff.min(t.len())];
            let mut i = 0;
            while i < opts.len() {
                match opts[i] {
                    0 => break,
                    1 => i += 1,
                    2 if i + 3 < opts.len() => {
                        mss = format!(" mss={}", u16::from_be_bytes([opts[i + 2], opts[i + 3]]));
                        i += 4;
                    }
                    _ if i + 1 < opts.len() && opts[i + 1] as usize >= 2 => i += opts[i + 1] as usize,
                    _ => break,
                }
            }
        }
        tcp = format!(
            " seq={seq} ack={ack} win={win} plen={payload} f={}{}{}{}{mss}",
            if flags & 0x02 != 0 { "S" } else { "" },
            if flags & 0x10 != 0 { "A" } else { "" },
            if flags & 0x01 != 0 { "F" } else { "" },
            if flags & 0x04 != 0 { "R" } else { "" },
        );
    }
    eprintln!(
        "PKT {dir} len={total} wire={} proto={proto} df={} mf={} off={off}{tcp}",
        pkt.len(),
        df as u8,
        mf as u8,
    );
}

// When smoltcp produces a packet (TCP SYN, data segment, UDP datagram, etc.),
// tokio-smoltcp hands it to this Sink, which forwards it on to the bridge for
// mixnet delivery.
//
// Delegates to the built-in Sink impl on futures::channel::mpsc::UnboundedSender;
// that impl already handles channel liveness checks (poll_ready) and disconnect
// (poll_close).
impl Sink<Vec<u8>> for NymAsyncDevice {
    type Error = io::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.tx)
            .poll_ready(cx)
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "bridge channel closed"))
    }

    fn start_send(mut self: Pin<&mut Self>, item: Vec<u8>) -> Result<(), Self::Error> {
        log_ipv4("OUT", &item);
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
