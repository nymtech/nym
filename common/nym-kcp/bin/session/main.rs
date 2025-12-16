use bytes::BytesMut;
use log::info;
use nym_kcp::{packet::KcpPacket, session::KcpSession};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create two KcpSessions, simulating two endpoints
    let mut local_sess = KcpSession::new(42);
    let mut remote_sess = KcpSession::new(42);

    // Set an MSS (max segment size) smaller than our data to force fragmentation
    local_sess.set_mtu(40);
    remote_sess.set_mtu(40);

    // Some data larger than 30 bytes to demonstrate multi-fragment
    let big_data = b"The quick brown fox jumps over the lazy dog. This is a test.";

    // --- LOCAL sends data ---
    info!(
        "Local: sending data: {:?}",
        String::from_utf8_lossy(big_data)
    );
    local_sess.send(big_data);

    // Update local session's logic at time=0
    local_sess.update(100);

    // LOCAL fetches outgoing (to be sent across the network)
    let outgoing_pkts = local_sess.fetch_outgoing();
    info!("Local: outgoing pkts: {:?}", outgoing_pkts);
    // Here you'd normally encrypt and send them. We’ll just encode them into a buffer.
    // Then that buffer is "transferred" to the remote side.
    let mut wire_buf = BytesMut::new();
    for pkt in &outgoing_pkts {
        pkt.encode(&mut wire_buf);
    }

    // --- REMOTE receives data ---
    // The remote side "decrypts" (here we just clone) and decodes
    let mut remote_in = wire_buf.clone();

    // Decode zero or more KcpPackets from remote_in
    while let Some(decoded_pkt) = KcpPacket::decode(&mut remote_in)? {
        info!(
            "Decoded packet, sn: {}, frg: {}",
            decoded_pkt.sn(),
            decoded_pkt.frg()
        );
        remote_sess.input(&decoded_pkt)?;
    }

    // Update remote session to process newly received data
    remote_sess.update(100);

    // The remote session likely generated ACK packets
    let ack_pkts = remote_sess.fetch_outgoing();

    // --- LOCAL receives ACKs ---
    // The local side decodes them
    let mut ack_buf = BytesMut::new();
    for pkt in &ack_pkts {
        pkt.encode(&mut ack_buf);
    }

    while let Some(decoded_pkt) = KcpPacket::decode(&mut ack_buf)? {
        local_sess.input(&decoded_pkt)?;
    }

    // Update local again with some arbitrary time, e.g. 50 ms later
    local_sess.update(100);

    // Just for completeness, local might produce more packets, though typically it's just empty now
    let _ = local_sess.fetch_outgoing();

    // --- REMOTE reads reassembled data ---

    let incoming = remote_sess.fetch_incoming();
    info!("Remote: incoming pkts: {:?}", incoming);

    Ok(())
}
