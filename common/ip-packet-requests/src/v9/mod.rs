pub const VERSION: u8 = 9;

/// Minimum nym-node release version that supports v9 (LP Stream framing).
/// Nodes running older versions will not understand LP-wrapped packets.
pub const MIN_RELEASE_VERSION: semver::Version = semver::Version::new(1, 30, 0);

// v9 uses the same wire format as v8. The version bump indicates
// the message was sent with LP framing (SphinxStream).
//
// Types are re-exported for deserialization/matching. Use the wrapper
// constructors below to create correctly-versioned packets — never
// manually set `protocol.version` or `response.version`.
pub use super::v8::{request, response};

/// Create a v9 connect request (version byte set to 9).
pub fn new_connect_request(buffer_timeout: Option<u64>) -> (request::IpPacketRequest, u64) {
    let (mut req, id) = request::IpPacketRequest::new_connect_request(buffer_timeout);
    req.protocol.version = VERSION;
    (req, id)
}

/// Create a v9 data request (version byte set to 9).
pub fn new_data_request(data: bytes::Bytes) -> request::IpPacketRequest {
    let mut req = request::IpPacketRequest::new_data_request(data);
    req.protocol.version = VERSION;
    req
}

/// Create a v9 IP packet response (version byte set to 9).
pub fn new_ip_packet_response(ip_packet: bytes::Bytes) -> response::IpPacketResponse {
    let mut resp = response::IpPacketResponse::new_ip_packet(ip_packet);
    resp.version = VERSION;
    resp
}
