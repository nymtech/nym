use nym_crypto::asymmetric::{ed25519::Ed25519RecoveryError, identity};

#[derive(thiserror::Error, Debug)]
pub enum SignatureError {
    #[error("signature is missing")]
    MissingSignature,

    #[error("failed to parse signature: {error}")]
    SignatureParseError {
        message: String,
        error: Ed25519RecoveryError,
    },

    #[error("failed to serialize request to binary: {message}")]
    RequestSerializationError {
        message: String,
        error: Box<bincode::ErrorKind>,
    },

    #[error("signature verification failed")]
    VerificationFailed {
        message: String,
        error: identity::SignatureError,
    },
}

pub trait SignedRequest {
    fn identity(&self) -> &identity::PublicKey;

    fn request(&self) -> Result<Vec<u8>, SignatureError>;

    fn signature(&self) -> Option<&Vec<u8>>;

    fn verify(&self) -> Result<(), SignatureError> {
        if let Some(signature) = self.signature() {
            let request_as_bytes = self.request()?;
            let signature = identity::Signature::from_bytes(signature).map_err(|error| {
                SignatureError::SignatureParseError {
                    message: "failed to parse signature".to_string(),
                    error,
                }
            })?;

            self.identity()
                .verify(request_as_bytes, &signature)
                .map_err(|error| SignatureError::VerificationFailed {
                    message: "signature verification failed".to_string(),
                    error,
                })
        } else {
            Err(SignatureError::MissingSignature)
        }
    }
}
