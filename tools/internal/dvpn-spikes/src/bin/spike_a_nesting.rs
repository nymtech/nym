//! Spike A — two-hop boringtun nesting parity (OpenSpec task 1.1).
//!
//! The `nym-vpn-client` reference performs the two-hop datapath inside Go/gVisor
//! (wireguard-go): the exit tunnel encrypts to the exit gateway; that ciphertext
//! is framed as an inner IP/UDP datagram (src = tunnel addr : fixed client port,
//! dst = exit endpoint) and handed to the entry tunnel, which encrypts to the
//! entry gateway. `boringtun` appears in the reference only in an offline
//! diagnostic, never in the datapath — so the *pure-Rust* nesting proven here is
//! genuinely new. This spike reproduces the mechanism entirely in-process with
//! two client `Tunn`s and two server `Tunn`s (the "gateways"), so it needs no
//! network, credentials, or live gateway — it isolates the one real unknown:
//! does exit-ciphertext → smoltcp IP/UDP frame → entry-`Tunn` → decap → parse →
//! exit-`Tunn` → decap round-trip losslessly?
//!
//! Run: `cargo run --bin spike_a_nesting` (exit code 0 = PASS).

use std::net::{Ipv4Addr, SocketAddrV4};

use boringtun::noise::{Tunn, TunnResult};
use boringtun::x25519::{PublicKey, StaticSecret};
use smoltcp::phy::ChecksumCapabilities;
use smoltcp::wire::{
    IpAddress, IpProtocol, Ipv4Packet, Ipv4Repr, UdpPacket, UdpRepr,
};

/// Reference fixed exit source port (`DEFAULT_EXIT_WG_CLIENT_PORT`,
/// two_hop_config.rs:17). In the reference it's a fallback behind a dynamic
/// port; here we pin it to assert the framing carries it verbatim.
const EXIT_WG_CLIENT_PORT: u16 = 54001;

const MAX: usize = 64 * 1024;

/// A pair of WireGuard peers (a "client" side and a "server"/gateway side)
/// that have completed a handshake in-process.
struct Peers {
    client: Tunn,
    server: Tunn,
}

fn new_keypair() -> (StaticSecret, PublicKey) {
    // boringtun re-exports x25519_dalek; StaticSecret::random needs an RNG, but
    // 0.7 exposes `random_from_rng`. Use OS randomness via getrandom through
    // x25519_dalek's `rand_core`. Simplest portable path: fill 32 bytes.
    let mut bytes = [0u8; 32];
    getrandom_fill(&mut bytes);
    let secret = StaticSecret::from(bytes);
    let public = PublicKey::from(&secret);
    (secret, public)
}

/// Minimal OS randomness without pulling an extra crate: read /dev/urandom.
fn getrandom_fill(buf: &mut [u8]) {
    use std::io::Read;
    let mut f = std::fs::File::open("/dev/urandom").expect("open /dev/urandom");
    f.read_exact(buf).expect("read /dev/urandom");
}

fn make_peers(index: u32) -> Peers {
    let (client_secret, client_public) = new_keypair();
    let (server_secret, server_public) = new_keypair();

    // No preshared key in this spike variant (the reference emits PSK optionally;
    // the datapath treats it as Option). No persistent keepalive (matches ref).
    let client = Tunn::new(client_secret, server_public, None, None, index, None);
    let server = Tunn::new(server_secret, client_public, None, None, index + 100, None);

    let mut peers = Peers { client, server };
    drive_handshake(&mut peers);
    peers
}

/// Shuttle handshake packets between client and server until the session is
/// established (no side has anything more to put on the wire).
fn drive_handshake(peers: &mut Peers) {
    let mut buf = vec![0u8; MAX];
    let init = match peers.client.format_handshake_initiation(&mut buf, false) {
        TunnResult::WriteToNetwork(p) => p.to_vec(),
        other => panic!("expected handshake initiation, got {}", describe(&other)),
    };

    let mut pkt = init;
    let mut to_server = true; // the initiation goes to the server first
    let mut guard = 0;
    loop {
        guard += 1;
        assert!(guard < 16, "handshake did not converge");
        let mut out = vec![0u8; MAX];
        let res = if to_server {
            peers.server.decapsulate(None, &pkt, &mut out)
        } else {
            peers.client.decapsulate(None, &pkt, &mut out)
        };
        match res {
            TunnResult::WriteToNetwork(p) => {
                pkt = p.to_vec();
                to_server = !to_server;
            }
            TunnResult::Done => break,
            other => panic!("handshake step failed: {}", describe(&other)),
        }
    }
    println!("  handshake established ({} round-trips)", guard);
}

/// Encapsulate one plaintext IP packet, draining boringtun's queue so we get the
/// single transport datagram out (the reference drain-loop pattern).
fn encapsulate(t: &mut Tunn, plaintext: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; MAX];
    match t.encapsulate(plaintext, &mut out) {
        TunnResult::WriteToNetwork(p) => p.to_vec(),
        other => panic!("encapsulate produced no network packet: {}", describe(&other)),
    }
}

/// Decapsulate one transport datagram, returning the recovered inner IP packet.
fn decapsulate(t: &mut Tunn, datagram: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; MAX];
    match t.decapsulate(None, datagram, &mut out) {
        TunnResult::WriteToTunnelV4(p, _) | TunnResult::WriteToTunnelV6(p, _) => p.to_vec(),
        other => panic!("decapsulate did not yield a tunnel packet: {}", describe(&other)),
    }
}

/// Build an IPv4/UDP packet carrying `payload` (the exit-tunnel ciphertext),
/// exactly the inner frame the entry `Tunn` must encrypt. Mirrors what
/// gVisor builds in the reference's in-tunnel UDP forwarder.
fn build_ipv4_udp(src: SocketAddrV4, dst: SocketAddrV4, payload: &[u8]) -> Vec<u8> {
    let udp_repr = UdpRepr {
        src_port: src.port(),
        dst_port: dst.port(),
    };
    let ip_repr = Ipv4Repr {
        src_addr: *src.ip(),
        dst_addr: *dst.ip(),
        next_header: IpProtocol::Udp,
        payload_len: udp_repr.header_len() + payload.len(),
        hop_limit: 64,
    };

    let mut buf = vec![0u8; ip_repr.buffer_len() + ip_repr.payload_len];
    let mut ip_pkt = Ipv4Packet::new_unchecked(&mut buf);
    ip_repr.emit(&mut ip_pkt, &ChecksumCapabilities::default());

    let mut udp_pkt = UdpPacket::new_unchecked(ip_pkt.payload_mut());
    udp_repr.emit(
        &mut udp_pkt,
        &IpAddress::Ipv4(*src.ip()),
        &IpAddress::Ipv4(*dst.ip()),
        payload.len(),
        |b| b.copy_from_slice(payload),
        &ChecksumCapabilities::default(),
    );
    buf
}

/// Parse an IPv4/UDP packet, returning (src, dst, udp_payload).
fn parse_ipv4_udp(bytes: &[u8]) -> (SocketAddrV4, SocketAddrV4, Vec<u8>) {
    let ip_pkt = Ipv4Packet::new_checked(bytes).expect("valid ipv4");
    assert_eq!(ip_pkt.next_header(), IpProtocol::Udp, "inner proto must be UDP");
    let src_ip = ip_pkt.src_addr();
    let dst_ip = ip_pkt.dst_addr();
    let udp_pkt = UdpPacket::new_checked(ip_pkt.payload()).expect("valid udp");
    let src = SocketAddrV4::new(src_ip, udp_pkt.src_port());
    let dst = SocketAddrV4::new(dst_ip, udp_pkt.dst_port());
    (src, dst, udp_pkt.payload().to_vec())
}

/// A representative application packet (an IPv4/UDP datagram) that the datapath
/// must carry losslessly. boringtun only emits `WriteToTunnelV4/V6` for payloads
/// that parse as IP, so using a real IP packet keeps the spike faithful.
fn app_packet() -> Vec<u8> {
    let src = SocketAddrV4::new(Ipv4Addr::new(10, 1, 0, 2), 5555);
    let dst = SocketAddrV4::new(Ipv4Addr::new(1, 1, 1, 1), 53);
    build_ipv4_udp(src, dst, b"the-quick-brown-fox-jumps-over-the-lazy-dog")
}

fn single_hop() {
    println!("[single-hop] one Tunn, no inner encapsulation");
    let mut hop = make_peers(1);
    let app = app_packet();
    let ct = encapsulate(&mut hop.client, &app);
    let recovered = decapsulate(&mut hop.server, &ct);
    assert_eq!(recovered, app, "single-hop payload mismatch");
    println!("  PASS: {} B app packet round-tripped ({} B on wire)", app.len(), ct.len());
}

fn two_hop() {
    println!("[two-hop] nested exit-inside-entry encapsulation");
    // "entry" gateway leg and "exit" gateway leg, each its own WG session.
    let mut entry = make_peers(2);
    let mut exit = make_peers(3);

    // Reference endpoints: the tunnel's own address:client_port -> exit endpoint.
    let tunnel_src = SocketAddrV4::new(Ipv4Addr::new(10, 2, 0, 2), EXIT_WG_CLIENT_PORT);
    let exit_endpoint = SocketAddrV4::new(Ipv4Addr::new(192, 0, 2, 40), 51820);

    let app = app_packet();

    // 1. Exit tunnel encrypts the application packet to the exit gateway.
    let c_exit = encapsulate(&mut exit.client, &app);

    // 2. Frame that ciphertext as an inner IP/UDP datagram (this is the layer
    //    the reference builds inside gVisor).
    let carrier = build_ipv4_udp(tunnel_src, exit_endpoint, &c_exit);

    // 3. Entry tunnel encrypts the carrier to the entry gateway.
    let c_entry = encapsulate(&mut entry.client, &carrier);
    println!("  on wire to entry gateway: {} B", c_entry.len());

    // --- network to entry gateway ---

    // 4. Entry gateway decapsulates -> recovers the carrier IP/UDP datagram.
    let carrier_recovered = decapsulate(&mut entry.server, &c_entry);
    let (src, dst, udp_payload) = parse_ipv4_udp(&carrier_recovered);

    assert_eq!(src.port(), EXIT_WG_CLIENT_PORT, "exit source port not preserved");
    assert_eq!(dst, exit_endpoint, "exit endpoint not preserved in inner frame");
    assert_eq!(udp_payload, c_exit, "exit ciphertext corrupted by nesting");

    // 5. Forwarded to exit gateway, which decapsulates -> original app packet.
    let app_recovered = decapsulate(&mut exit.server, &udp_payload);
    assert_eq!(app_recovered, app, "two-hop payload mismatch");

    println!(
        "  PASS: {} B app packet survived exit->IP/UDP(:{})->entry nesting",
        app.len(),
        EXIT_WG_CLIENT_PORT
    );
}

fn describe(r: &TunnResult) -> String {
    match r {
        TunnResult::Done => "Done".into(),
        TunnResult::Err(e) => format!("Err({e:?})"),
        TunnResult::WriteToNetwork(p) => format!("WriteToNetwork({} B)", p.len()),
        TunnResult::WriteToTunnelV4(p, a) => format!("WriteToTunnelV4({} B, {a})", p.len()),
        TunnResult::WriteToTunnelV6(p, a) => format!("WriteToTunnelV6({} B, {a})", p.len()),
    }
}

fn main() {
    println!("== Spike A: two-hop boringtun nesting parity ==");
    single_hop();
    two_hop();
    println!("== Spike A PASS ==");
}
