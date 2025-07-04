use nym_crypto::asymmetric::ed25519::{PublicKey, Signature};
use serde::{Deserialize, Serialize};

pub mod get_testrun {
    use crate::auth::SignedRequest;

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
    use crate::auth::SignedRequest;

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

pub mod submit_results_v2 {
    use crate::auth::SignedRequest;

    use super::*;
    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct Payload {
        pub probe_result: String,
        pub agent_public_key: PublicKey,
        pub assigned_at_utc: i64,
        pub gateway_identity_key: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct SubmitResultsV2 {
        pub payload: Payload,
        pub signature: Signature,
    }

    impl SignedRequest for SubmitResultsV2 {
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
