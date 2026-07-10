// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

//! The userspace WireGuard engine (boringtun).
//!
//! Owns the one (single-hop) or two (two-hop) `Tunn`s and implements the nesting
//! proven in conformance spike A. Every method returns an [`EngineOutput`]
//! splitting work into inner IP packets destined for the smol-core stack and
//! outer WireGuard packets destined for the active transport. The engine is
//! driven from a single task, so the `Tunn`s need no locking.
//!
//! Handshake note: the entry handshake runs directly; the exit handshake is
//! tunnelled through the entry `Tunn`. boringtun drives retransmission via
//! `update_timers`, so the timer pump re-kicks the exit handshake until the
//! session establishes. End-to-end handshake success is validated against a
//! live gateway; the encap/decap/framing correctness is covered by spike A.

use std::net::SocketAddrV4;

use boringtun::noise::{Tunn, TunnResult};
use boringtun::x25519::{PublicKey, StaticSecret};

use crate::config::PeerConfig;
use crate::framing::{build_ipv4_udp, parse_ipv4_udp};

const MAX: usize = 65535;

/// The split output of an engine operation.
#[derive(Default)]
pub(crate) struct EngineOutput {
    /// Decrypted inner IP packets to hand to the smol-core stack.
    pub to_stack: Vec<Vec<u8>>,
    /// Outer WireGuard packets to send over the active transport.
    pub to_network: Vec<Vec<u8>>,
}

fn make_tunn(peer: &PeerConfig, index: u32) -> Tunn {
    let secret = StaticSecret::from(peer.client_private_key);
    let public = PublicKey::from(peer.gateway_public_key);
    Tunn::new(secret, public, peer.preshared_key, None, index, None)
}

/// Encapsulate one plaintext packet; returns the single transport datagram if
/// boringtun produced one (data or a handshake message).
fn encap(tunn: &mut Tunn, plaintext: &[u8]) -> Option<Vec<u8>> {
    let mut out = vec![0u8; MAX];
    match tunn.encapsulate(plaintext, &mut out) {
        TunnResult::WriteToNetwork(p) => Some(p.to_vec()),
        _ => None,
    }
}

/// Decapsulate one transport datagram, draining boringtun's queue. Returns
/// (inner packets recovered, network responses to send back).
fn decap(tunn: &mut Tunn, datagram: &[u8]) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
    let mut inner = Vec::new();
    let mut net = Vec::new();
    let mut out = vec![0u8; MAX];
    match tunn.decapsulate(None, datagram, &mut out) {
        TunnResult::WriteToTunnelV4(p, _) | TunnResult::WriteToTunnelV6(p, _) => {
            inner.push(p.to_vec());
        }
        TunnResult::WriteToNetwork(p) => {
            net.push(p.to_vec());
            // Drain any further queued network packets.
            loop {
                let mut o2 = vec![0u8; MAX];
                match tunn.decapsulate(None, &[], &mut o2) {
                    TunnResult::WriteToNetwork(p) => net.push(p.to_vec()),
                    _ => break,
                }
            }
        }
        _ => {}
    }
    (inner, net)
}

/// Timer maintenance; returns the single packet boringtun wants sent, if any.
fn timer(tunn: &mut Tunn) -> Option<Vec<u8>> {
    let mut out = vec![0u8; MAX];
    match tunn.update_timers(&mut out) {
        TunnResult::WriteToNetwork(p) => Some(p.to_vec()),
        _ => None,
    }
}

fn handshake_init(tunn: &mut Tunn) -> Option<Vec<u8>> {
    let mut out = vec![0u8; MAX];
    match tunn.format_handshake_initiation(&mut out, false) {
        TunnResult::WriteToNetwork(p) => Some(p.to_vec()),
        _ => None,
    }
}

// A single long-lived instance per tunnel; the inter-variant size difference is
// immaterial (no bulk storage), so boxing would only add indirection.
#[allow(clippy::large_enum_variant)]
pub(crate) enum WgEngine {
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

impl WgEngine {
    pub(crate) fn single_hop(peer: &PeerConfig) -> Self {
        WgEngine::SingleHop {
            tunn: make_tunn(peer, 0),
        }
    }

    pub(crate) fn two_hop(
        entry: &PeerConfig,
        exit: &PeerConfig,
        tunnel_src: SocketAddrV4,
        exit_endpoint: SocketAddrV4,
    ) -> Self {
        WgEngine::TwoHop {
            entry: make_tunn(entry, 0),
            exit: make_tunn(exit, 1),
            tunnel_src,
            exit_endpoint,
        }
    }

    /// Frame + entry-encapsulate an exit-bound packet (two-hop only).
    fn wrap_for_exit(
        entry: &mut Tunn,
        tunnel_src: SocketAddrV4,
        exit_endpoint: SocketAddrV4,
        exit_packet: &[u8],
    ) -> Option<Vec<u8>> {
        let carrier = build_ipv4_udp(tunnel_src, exit_endpoint, exit_packet);
        encap(entry, &carrier)
    }

    /// Encrypt one application IP packet from the stack.
    pub(crate) fn encapsulate_app(&mut self, app: &[u8]) -> EngineOutput {
        let mut out = EngineOutput::default();
        match self {
            WgEngine::SingleHop { tunn } => {
                if let Some(p) = encap(tunn, app) {
                    out.to_network.push(p);
                }
            }
            WgEngine::TwoHop {
                entry,
                exit,
                tunnel_src,
                exit_endpoint,
            } => {
                if let Some(c_exit) = encap(exit, app) {
                    if let Some(c_entry) =
                        Self::wrap_for_exit(entry, *tunnel_src, *exit_endpoint, &c_exit)
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
        let mut out = EngineOutput::default();
        match self {
            WgEngine::SingleHop { tunn } => {
                let (inner, net) = decap(tunn, wg);
                out.to_stack = inner;
                out.to_network = net;
            }
            WgEngine::TwoHop {
                entry,
                exit,
                tunnel_src,
                exit_endpoint,
            } => {
                let (carriers, entry_net) = decap(entry, wg);
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
                    let (app_pkts, exit_net) = decap(exit, &parsed.payload);
                    out.to_stack.extend(app_pkts);
                    for c_exit in exit_net {
                        if let Some(c_entry) =
                            Self::wrap_for_exit(entry, *tunnel_src, *exit_endpoint, &c_exit)
                        {
                            out.to_network.push(c_entry);
                        }
                    }
                }
            }
        }
        out
    }

    /// Timer maintenance (keepalive / handshake / rekey).
    pub(crate) fn update_timers(&mut self) -> EngineOutput {
        let mut out = EngineOutput::default();
        match self {
            WgEngine::SingleHop { tunn } => {
                if let Some(p) = timer(tunn) {
                    out.to_network.push(p);
                }
            }
            WgEngine::TwoHop {
                entry,
                exit,
                tunnel_src,
                exit_endpoint,
            } => {
                if let Some(p) = timer(entry) {
                    out.to_network.push(p);
                }
                if let Some(c_exit) = timer(exit) {
                    if let Some(c_entry) =
                        Self::wrap_for_exit(entry, *tunnel_src, *exit_endpoint, &c_exit)
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
        match self {
            WgEngine::SingleHop { tunn } => {
                if let Some(p) = handshake_init(tunn) {
                    out.to_network.push(p);
                }
            }
            WgEngine::TwoHop {
                entry,
                exit,
                tunnel_src,
                exit_endpoint,
            } => {
                if let Some(p) = handshake_init(entry) {
                    out.to_network.push(p);
                }
                // Kick the (tunnelled) exit handshake; if the entry session is
                // not up yet this queues behind the entry handshake and the
                // timer pump retransmits until it establishes.
                if let Some(c_exit) = handshake_init(exit) {
                    if let Some(c_entry) =
                        Self::wrap_for_exit(entry, *tunnel_src, *exit_endpoint, &c_exit)
                    {
                        out.to_network.push(c_entry);
                    }
                }
            }
        }
        out
    }
}
