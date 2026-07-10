// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The userspace WireGuard engine (boringtun).
//!
//! Owns the one (single-hop) or two (two-hop) `Tunn`s and implements the two-hop
//! nesting. Every method returns an [`EngineOutput`]
//! splitting work into inner IP packets destined for the smol-core stack and
//! outer WireGuard packets destined for the active transport. The engine is
//! driven from a single task, so the `Tunn`s need no locking — and, for the same
//! reason, it owns one reusable scratch buffer that boringtun encrypts/decrypts
//! into, rather than allocating (and zero-filling) a fresh 64 KiB buffer per packet.
//!
//! Handshake note: the entry handshake runs directly; the exit handshake is
//! tunnelled through the entry `Tunn`. boringtun drives retransmission via
//! `update_timers`, so the timer pump re-kicks the exit handshake until the
//! session establishes. Handshake success and encap/decap/framing correctness
//! are validated end-to-end against a live gateway (see the crate examples).

use std::net::SocketAddrV4;

use boringtun::noise::{Tunn, TunnResult};

use crate::config::PeerConfig;
use crate::framing::{build_ipv4_udp, parse_ipv4_udp};

/// Size of the engine's reusable encrypt/decrypt scratch buffer: the maximum a WireGuard/UDP
/// datagram can be (64 KiB). It is allocated once per engine and reused for every packet, so — unlike
/// the previous per-call `vec![0u8; 65535]` — there is no per-packet allocation or zero-fill. Sized
/// to the maximum so no packet is ever dropped for want of buffer space, regardless of the
/// configured MTU (which callers can set arbitrarily large).
const SCRATCH_LEN: usize = 65535;

/// The split output of an engine operation.
#[derive(Default)]
pub(crate) struct EngineOutput {
    /// Decrypted inner IP packets to hand to the smol-core stack.
    pub to_stack: Vec<Vec<u8>>,
    /// Outer WireGuard packets to send over the active transport.
    pub to_network: Vec<Vec<u8>>,
}

fn make_tunn(peer: &PeerConfig, index: u32) -> Tunn {
    Tunn::new(
        peer.client_private_key.inner().clone(),
        peer.gateway_public_key.inner(),
        peer.preshared_key,
        None,
        index,
        None,
    )
}

/// Encapsulate one plaintext packet into `scratch`; returns the single transport datagram if
/// boringtun produced one (data or a handshake message).
fn encap(tunn: &mut Tunn, scratch: &mut [u8], plaintext: &[u8]) -> Option<Vec<u8>> {
    match tunn.encapsulate(plaintext, scratch) {
        TunnResult::WriteToNetwork(p) => Some(p.to_vec()),
        TunnResult::Err(e) => {
            tracing::warn!("wireguard encapsulate error: {e:?}");
            None
        }
        _ => None,
    }
}

/// Decapsulate one transport datagram into `scratch`, draining boringtun's queue. Returns
/// (inner packets recovered, network responses to send back).
fn decap(tunn: &mut Tunn, scratch: &mut [u8], datagram: &[u8]) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
    let mut inner = Vec::new();
    let mut net = Vec::new();
    // Each `decapsulate` borrows `scratch` for the lifetime of its result, so copy the result out
    // (releasing the borrow at the statement end) before the next call reuses the same buffer.
    let queued = match tunn.decapsulate(None, datagram, scratch) {
        TunnResult::WriteToTunnelV4(p, _) | TunnResult::WriteToTunnelV6(p, _) => {
            inner.push(p.to_vec());
            false
        }
        TunnResult::WriteToNetwork(p) => {
            net.push(p.to_vec());
            true
        }
        // Surface rejected/undecryptable datagrams (e.g. a handshake response
        // that fails validation) instead of dropping them silently — key
        // evidence when a tunnel comes up but no data flows.
        TunnResult::Err(e) => {
            tracing::warn!("wireguard decapsulate error: {e:?}");
            false
        }
        _ => false,
    };
    if queued {
        // Drain any further queued network packets.
        while let TunnResult::WriteToNetwork(p) = tunn.decapsulate(None, &[], scratch) {
            net.push(p.to_vec());
        }
    }
    (inner, net)
}

/// Timer maintenance; returns the single packet boringtun wants sent, if any.
fn timer(tunn: &mut Tunn, scratch: &mut [u8]) -> Option<Vec<u8>> {
    match tunn.update_timers(scratch) {
        TunnResult::WriteToNetwork(p) => Some(p.to_vec()),
        _ => None,
    }
}

fn handshake_init(tunn: &mut Tunn, scratch: &mut [u8]) -> Option<Vec<u8>> {
    match tunn.format_handshake_initiation(scratch, false) {
        TunnResult::WriteToNetwork(p) => Some(p.to_vec()),
        _ => None,
    }
}

/// Frame + entry-encapsulate an exit-bound packet (two-hop only).
fn wrap_for_exit(
    entry: &mut Tunn,
    scratch: &mut [u8],
    tunnel_src: SocketAddrV4,
    exit_endpoint: SocketAddrV4,
    exit_packet: &[u8],
) -> Option<Vec<u8>> {
    let carrier = build_ipv4_udp(tunnel_src, exit_endpoint, exit_packet);
    encap(entry, scratch, &carrier)
}

// A single long-lived instance per tunnel; the inter-variant size difference is
// immaterial (no bulk storage), so boxing would only add indirection.
#[allow(clippy::large_enum_variant)]
enum Inner {
    SingleHop {
        tunn: Tunn,
    },
    TwoHop {
        entry: Tunn,
        exit: Tunn,
        /// Assigned exit tunnel address : fixed exit client port.
        tunnel_src: SocketAddrV4,
        /// The real exit gateway endpoint the inner frame targets.
        exit_endpoint: SocketAddrV4,
    },
}

/// The userspace WireGuard engine: the `Tunn`(s) plus a reusable encrypt/decrypt scratch buffer.
pub(crate) struct WgEngine {
    inner: Inner,
    scratch: Box<[u8]>,
    // One-shot diagnostic markers (logged at info): they localize a dead
    // datapath from a single default-log run — no inbound datagrams at all vs.
    // entry handshake never completing vs. exit handshake never completing.
    saw_inbound: bool,
    entry_established: bool,
    exit_established: bool,
}

impl WgEngine {
    pub(crate) fn single_hop(peer: &PeerConfig) -> Self {
        WgEngine {
            inner: Inner::SingleHop {
                tunn: make_tunn(peer, 0),
            },
            scratch: vec![0u8; SCRATCH_LEN].into_boxed_slice(),
            saw_inbound: false,
            entry_established: false,
            exit_established: false,
        }
    }

    pub(crate) fn two_hop(
        entry: &PeerConfig,
        exit: &PeerConfig,
        tunnel_src: SocketAddrV4,
        exit_endpoint: SocketAddrV4,
    ) -> Self {
        WgEngine {
            inner: Inner::TwoHop {
                entry: make_tunn(entry, 0),
                exit: make_tunn(exit, 1),
                tunnel_src,
                exit_endpoint,
            },
            scratch: vec![0u8; SCRATCH_LEN].into_boxed_slice(),
            saw_inbound: false,
            entry_established: false,
            exit_established: false,
        }
    }

    /// Per-hop establishment state: `(entry, exit)` where `exit` is `None` for
    /// single-hop tunnels. Drives `Tunnel::await_established`.
    pub(crate) fn establishment(&self) -> (bool, Option<bool>) {
        match &self.inner {
            Inner::SingleHop { .. } => (self.entry_established, None),
            Inner::TwoHop { .. } => (self.entry_established, Some(self.exit_established)),
        }
    }

    /// Log the one-shot session-establishment markers after processing inbound
    /// datagrams. `stats().0` (time since last handshake) flips to `Some` once
    /// a hop's handshake completes.
    fn note_progress(&mut self) {
        match &self.inner {
            Inner::SingleHop { tunn } => {
                if !self.entry_established && tunn.stats().0.is_some() {
                    self.entry_established = true;
                    tracing::info!("wireguard session established");
                }
            }
            Inner::TwoHop { entry, exit, .. } => {
                if !self.entry_established && entry.stats().0.is_some() {
                    self.entry_established = true;
                    tracing::info!("entry-hop wireguard session established");
                }
                if !self.exit_established && exit.stats().0.is_some() {
                    self.exit_established = true;
                    tracing::info!("exit-hop wireguard session established");
                }
            }
        }
    }

    /// Encrypt one application IP packet from the stack.
    pub(crate) fn encapsulate_app(&mut self, app: &[u8]) -> EngineOutput {
        let mut out = EngineOutput::default();
        let WgEngine { inner, scratch, .. } = self;
        match inner {
            Inner::SingleHop { tunn } => {
                if let Some(p) = encap(tunn, scratch, app) {
                    out.to_network.push(p);
                }
            }
            Inner::TwoHop {
                entry,
                exit,
                tunnel_src,
                exit_endpoint,
            } => {
                if let Some(c_exit) = encap(exit, scratch, app) {
                    if let Some(c_entry) =
                        wrap_for_exit(entry, scratch, *tunnel_src, *exit_endpoint, &c_exit)
                    {
                        out.to_network.push(c_entry);
                    }
                }
            }
        }
        out
    }

    /// Decrypt one incoming outer WireGuard packet from the transport.
    pub(crate) fn decapsulate_incoming(&mut self, wg: &[u8]) -> EngineOutput {
        if !self.saw_inbound {
            self.saw_inbound = true;
            tracing::info!("first datagram received from the entry transport");
        }
        let mut out = EngineOutput::default();
        let WgEngine { inner, scratch, .. } = self;
        match inner {
            Inner::SingleHop { tunn } => {
                let (inner_pkts, net) = decap(tunn, scratch, wg);
                out.to_stack = inner_pkts;
                out.to_network = net;
            }
            Inner::TwoHop {
                entry,
                exit,
                tunnel_src,
                exit_endpoint,
            } => {
                let (carriers, entry_net) = decap(entry, scratch, wg);
                out.to_network.extend(entry_net); // entry handshake responses (direct)
                for carrier in carriers {
                    let Some(parsed) = parse_ipv4_udp(&carrier) else {
                        continue;
                    };
                    // Defensive: the inbound inner frame must originate from the
                    // exit endpoint (the entry gateway NATs the exit's reply back
                    // to us with the exit's address as source).
                    if parsed.src != *exit_endpoint {
                        continue;
                    }
                    // parsed.payload is the exit-gateway WireGuard packet.
                    let (app_pkts, exit_net) = decap(exit, scratch, &parsed.payload);
                    out.to_stack.extend(app_pkts);
                    for c_exit in exit_net {
                        if let Some(c_entry) =
                            wrap_for_exit(entry, scratch, *tunnel_src, *exit_endpoint, &c_exit)
                        {
                            out.to_network.push(c_entry);
                        }
                    }
                }
            }
        }
        self.note_progress();
        out
    }

    /// Timer maintenance (keepalive / handshake / rekey).
    pub(crate) fn update_timers(&mut self) -> EngineOutput {
        let mut out = EngineOutput::default();
        let WgEngine { inner, scratch, .. } = self;
        match inner {
            Inner::SingleHop { tunn } => {
                if let Some(p) = timer(tunn, scratch) {
                    out.to_network.push(p);
                }
            }
            Inner::TwoHop {
                entry,
                exit,
                tunnel_src,
                exit_endpoint,
            } => {
                if let Some(p) = timer(entry, scratch) {
                    out.to_network.push(p);
                }
                if let Some(c_exit) = timer(exit, scratch) {
                    if let Some(c_entry) =
                        wrap_for_exit(entry, scratch, *tunnel_src, *exit_endpoint, &c_exit)
                    {
                        out.to_network.push(c_entry);
                    }
                }
            }
        }
        out
    }

    /// Kick the initial handshake(s).
    pub(crate) fn initiate_handshakes(&mut self) -> EngineOutput {
        let mut out = EngineOutput::default();
        let WgEngine { inner, scratch, .. } = self;
        match inner {
            Inner::SingleHop { tunn } => {
                if let Some(p) = handshake_init(tunn, scratch) {
                    out.to_network.push(p);
                }
            }
            Inner::TwoHop {
                entry,
                exit,
                tunnel_src,
                exit_endpoint,
            } => {
                if let Some(p) = handshake_init(entry, scratch) {
                    out.to_network.push(p);
                }
                // Kick the (tunnelled) exit handshake; if the entry session is
                // not up yet this queues behind the entry handshake and the
                // timer pump retransmits until it establishes.
                if let Some(c_exit) = handshake_init(exit, scratch) {
                    if let Some(c_entry) =
                        wrap_for_exit(entry, scratch, *tunnel_src, *exit_endpoint, &c_exit)
                    {
                        out.to_network.push(c_entry);
                    }
                }
            }
        }
        out
    }
}
