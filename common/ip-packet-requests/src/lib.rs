use nym_crypto::asymmetric::ed25519;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::Duration;
use time::OffsetDateTime;

pub use v6::request;
pub use v6::response;

pub mod codec;
pub mod v6;
pub mod v7;
pub mod v8;

// version 3: initial version
// version 4: IPv6 support
// version 5: Add severity level to info response
// version 6: Increase the available IPs
// version 7: Add signature support (for the future)
// version 8: anonymous sends

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IpPair {
    pub ipv4: Ipv4Addr,
    pub ipv6: Ipv6Addr,
}

impl IpPair {
    pub fn new(ipv4: Ipv4Addr, ipv6: Ipv6Addr) -> Self {
        IpPair { ipv4, ipv6 }
    }
}

impl Display for IpPair {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "IPv4: {}, IPv6: {}", self.ipv4, self.ipv6)
    }
}

fn make_bincode_serializer() -> impl bincode::Options {
    use bincode::Options;
    bincode::DefaultOptions::new()
        .with_big_endian()
        .with_varint_encoding()
}

// For reply protection, if a request is older than this, it will be rejected
const MAX_REQUEST_AGE: Duration = Duration::from_secs(10);

#[derive(thiserror::Error, Debug)]
pub enum SignatureError {
    #[error("signature is missing")]
    MissingSignature,

    #[error("failed to serialize request to binary: {message}")]
    RequestSerializationError {
        message: String,
        error: Box<bincode::ErrorKind>,
    },

    #[error("signature verification failed: request out of date")]
    RequestOutOfDate,

    #[error("signature verification failed")]
    VerificationFailed {
        message: String,
        error: ed25519::SignatureError,
    },
}

pub trait SignedRequest {
    fn identity(&self) -> &ed25519::PublicKey;

    fn request(&self) -> Result<Vec<u8>, SignatureError>;

    fn signature(&self) -> Option<&ed25519::Signature>;

    fn timestamp(&self) -> OffsetDateTime;

    fn verify(&self) -> Result<(), SignatureError> {
        if let Some(signature) = self.signature() {
            // First check that the request is recent enough
            if OffsetDateTime::now_utc() - self.timestamp() > MAX_REQUEST_AGE {
                return Err(SignatureError::RequestOutOfDate);
            }

            let request_as_bytes = self.request()?;

            self.identity()
                .verify(request_as_bytes, signature)
                .map_err(|error| SignatureError::VerificationFailed {
                    message: "signature verification failed".to_string(),
                    error,
                })
        } else {
            Err(SignatureError::MissingSignature)
        }
    }
}
