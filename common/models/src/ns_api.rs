use nym_crypto::asymmetric::ed25519::{PublicKey, Signature, SignatureError};
use serde::{Deserialize, Serialize};

pub mod get_testrun {
    use super::*;
    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct Payload {
        pub agent_public_key: PublicKey,
        pub timestamp: i64,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct GetTestrunRequest {
        pub payload: Payload,
        pub signature: Signature,
    }

    impl SignedRequest for GetTestrunRequest {
        type Payload = Payload;

        fn public_key(&self) -> &PublicKey {
            &self.payload.agent_public_key
        }

        fn signature(&self) -> &Signature {
            &self.signature
        }

        fn payload(&self) -> &Self::Payload {
            &self.payload
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestrunAssignment {
    pub testrun_id: i64,
    pub assigned_at_utc: i64,
    pub gateway_identity_key: String,
}

pub mod submit_results {
    use super::*;
    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct Payload {
        pub probe_result: String,
        pub agent_public_key: PublicKey,
        pub assigned_at_utc: i64,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct SubmitResults {
        pub payload: Payload,
        pub signature: Signature,
    }

    impl SignedRequest for SubmitResults {
        type Payload = Payload;

        fn public_key(&self) -> &PublicKey {
            &self.payload.agent_public_key
        }

        fn signature(&self) -> &Signature {
            &self.signature
        }

        fn payload(&self) -> &Self::Payload {
            &self.payload
        }
    }
}

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
