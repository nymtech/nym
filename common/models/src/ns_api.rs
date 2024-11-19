use nym_crypto::asymmetric::ed25519::{PublicKey, Signature};
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
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestrunAssignment {
    pub testrun_id: i64,
    pub gateway_identity_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SubmitResults {
    pub message: String,
    pub signature: Signature,
}
