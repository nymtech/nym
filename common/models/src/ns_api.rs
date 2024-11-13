use nym_crypto::asymmetric::ed25519::{PublicKey, Signature};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestrunAssignment {
    pub testrun_id: i64,
    pub gateway_identity_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SubmitResults {
    pub message: String,
    pub signature: Signature,
    pub public_key: PublicKey,
}
