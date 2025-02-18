use std::time::Duration;

use nym_crypto::asymmetric::ed25519;
use time::OffsetDateTime;

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

    fn request_as_bytes(&self) -> Result<Vec<u8>, SignatureError>;

    // The signature is Option in v7. Once v7 is phased out we can change this to remove the
    // Option
    fn signature(&self) -> Option<&ed25519::Signature>;

    fn timestamp(&self) -> OffsetDateTime;

    fn verify(&self) -> Result<(), SignatureError> {
        if let Some(signature) = self.signature() {
            // First check that the request is recent enough
            if OffsetDateTime::now_utc() - self.timestamp() > MAX_REQUEST_AGE {
                return Err(SignatureError::RequestOutOfDate);
            }

            let request_as_bytes = self.request_as_bytes()?;

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
