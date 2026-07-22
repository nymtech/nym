pub const VERSION: u8 = 10;

/// Minimum nym-node release that supports v10. Older nodes fall back to v9.
pub const MIN_RELEASE_VERSION: semver::Version = semver::Version::new(1, 37, 0);

// Same request wire format as v8; only the connect response differs (it carries
// the IPR's MTU). Use the wrapper constructors below to set the version byte.
pub use super::v8::request;
pub mod response;

/// Create a v10 connect request (version byte set to 10).
pub fn new_connect_request(buffer_timeout: Option<u64>) -> (request::IpPacketRequest, u64) {
    let (mut req, id) = request::IpPacketRequest::new_connect_request(buffer_timeout);
    req.protocol.version = VERSION;
    (req, id)
}

/// Create a v10 data request (version byte set to 10).
pub fn new_data_request(data: bytes::Bytes) -> request::IpPacketRequest {
    let mut req = request::IpPacketRequest::new_data_request(data);
    req.protocol.version = VERSION;
    req
}

/// Create a v10 IP packet response (version byte set to 10).
pub fn new_ip_packet_response(ip_packet: bytes::Bytes) -> response::IpPacketResponse {
    let mut resp = response::IpPacketResponse::new_ip_packet(ip_packet);
    resp.version = VERSION;
    resp
}
