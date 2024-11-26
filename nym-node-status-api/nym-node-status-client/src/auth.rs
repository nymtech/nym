use nym_crypto::asymmetric::ed25519::{PublicKey, Signature, SignatureError};

pub trait SignedRequest {
    type Payload: serde::Serialize;

    fn public_key(&self) -> &PublicKey;
    fn signature(&self) -> &Signature;
    fn payload(&self) -> &Self::Payload;
}

pub trait VerifiableRequest: SignedRequest {
    type Error: From<bincode::Error> + From<SignatureError>;

    fn verify_signature(&self) -> Result<(), Self::Error> {
        bincode::serialize(self.payload())
            .map_err(Self::Error::from)
            .and_then(|serialized| {
                self.public_key()
                    .verify(serialized, self.signature())
                    .map_err(Self::Error::from)
            })
    }
}

impl<T> VerifiableRequest for T
where
    T: SignedRequest,
{
    type Error = anyhow::Error;
}
