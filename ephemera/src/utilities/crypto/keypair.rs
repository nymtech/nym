use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum KeyPairError {
    #[error("Failed to encode: {0}")]
    Encoding(String),
    #[error("Failed to decode: {0}")]
    Decoding(String),
    #[error("Invalid signature")]
    Signature(String),
    #[error("Signing failed: {0}")]
    Signing(String),
}

pub trait EphemeraPublicKey {
    type Signature: AsRef<[u8]>;

    /// Returns raw bytes of the public key
    ///
    /// # Returns
    /// * `Vec<u8>` - raw bytes of the public key
    fn to_bytes(&self) -> Vec<u8>;

    /// Parses public key from raw bytes
    ///
    /// # Arguments
    /// * `raw` - raw bytes of the public key
    ///
    /// # Returns
    /// * `Result<Self, KeyPairError>` - public key or error
    ///
    /// # Errors
    /// * `KeyPairError::Decoding` - if bytes are not valid
    fn from_bytes(bytes: &[u8]) -> Result<Self, KeyPairError>
    where
        Self: Sized;

    /// Verifies the signature of the message using the public key
    ///
    /// # Arguments
    /// * `msg` - message to verify
    /// * `signature` - signature of the message
    ///
    /// # Returns
    /// * `bool` - true if the signature is valid, false otherwise
    fn verify<M: AsRef<[u8]>>(&self, msg: &M, signature: &Self::Signature) -> bool;

    /// Returns base58 encoded public key
    ///
    /// # Returns
    /// * `String` - base58 encoded public key
    fn to_base58(&self) -> String {
        bs58::encode(self.to_bytes()).into_string()
    }

    /// Parses public key from base58 encoded string
    ///
    /// # Arguments
    /// * `base58` - base58 encoded public key
    ///
    /// # Returns
    /// * `Result<Self, KeyPairError>` - public key or error
    ///
    /// # Errors
    /// * `KeyPairError::Decoding` - if bytes are not valid
    fn from_base58(base58: &str) -> Result<Self, KeyPairError>
    where
        Self: Sized,
    {
        let raw = bs58::decode(base58)
            .into_vec()
            .map_err(|err| KeyPairError::Decoding(err.to_string()))?;
        Self::from_bytes(&raw)
    }
}

#[allow(clippy::module_name_repetitions)]
pub trait EphemeraKeypair {
    type Signature;
    type PublicKey;

    /// Generates a new keypair. Depending on the implementation, the seed may be used to
    /// add entropy to the key generation process.
    fn generate(seed: Option<Vec<u8>>) -> Self;

    /// Signs a message with the private key
    ///
    /// # Arguments
    /// * `msg` - message to sign
    ///
    /// # Returns
    /// * `Result<Self::Signature, KeyPairError>` - signature or error
    ///
    /// # Errors
    /// * `KeyPairError::Signing` - if the message cannot be encoded
    fn sign<M: AsRef<[u8]>>(&self, msg: &M) -> Result<Self::Signature, KeyPairError>;

    /// Verifies the signature of the message using the related public key
    ///
    /// # Arguments
    /// * `msg` - message to verify
    /// * `signature` - signature of the message
    ///
    /// # Returns
    /// * `bool` - true if the signature is valid, false otherwise
    fn verify<M: AsRef<[u8]>>(&self, msg: &M, signature: &Self::Signature) -> bool;

    /// Returns raw bytes of the keypair
    ///
    /// # Returns
    /// * `Vec<u8>` - raw bytes of the keypair
    fn to_bytes(&self) -> Vec<u8>;

    /// Parses keypair from bytes
    ///
    /// # Arguments
    /// * `raw` - raw bytes of the keypair
    ///
    /// # Returns
    /// * `Result<Self, KeyPairError>` - keypair or error
    ///
    /// # Errors
    /// * `KeyPairError::Decoding` - if bytes are not valid
    fn from_bytes(raw: &[u8]) -> Result<Self, KeyPairError>
    where
        Self: Sized;

    /// Returns related public key of the keypair
    fn public_key(&self) -> Self::PublicKey;

    /// Returns base58 encoded keypair
    ///
    /// # Returns
    /// * `String` - base58 encoded keypair
    fn to_base58(&self) -> String {
        bs58::encode(self.to_bytes()).into_string()
    }

    /// Parses keypair from base58 encoded string
    ///
    /// # Arguments
    /// * `base58` - base58 encoded keypair
    ///
    /// # Returns
    /// * `Result<Self, KeyPairError>` - keypair or error
    ///
    /// # Errors
    /// * `KeyPairError::Decoding` - if bytes are not valid base58 encoded string
    fn from_base58(base58: &str) -> Result<Self, KeyPairError>
    where
        Self: Sized,
    {
        let raw = bs58::decode(base58)
            .into_vec()
            .map_err(|err| KeyPairError::Decoding(err.to_string()))?;
        Self::from_bytes(&raw)
    }
}
