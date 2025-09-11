use nym_credentials::ecash::bandwidth::serialiser::VersionSerialised;
use nym_credentials::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures, EpochVerificationKey,
    IssuedTicketBook,
};
use nym_crypto::asymmetric::ed25519::{PublicKey, Signature};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

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

#[derive(Serialize, Deserialize)]
pub struct AttachedTicket {
    pub ticketbook: VersionSerialised<IssuedTicketBook>,
    pub usable_index: u32,
}

#[derive(Deserialize, Serialize)]
pub struct AttachedTicketMaterials {
    pub coin_indices_signatures: Option<VersionSerialised<AggregatedCoinIndicesSignatures>>,

    pub expiration_date_signatures: Option<VersionSerialised<AggregatedExpirationDateSignatures>>,

    pub master_verification_key: Option<VersionSerialised<EpochVerificationKey>>,

    // we need one ticket per type
    pub attached_tickets: Vec<AttachedTicket>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TestrunAssignment {
    pub testrun_id: i32,
    pub assigned_at_utc: i64,
    pub gateway_identity_key: String,
}

#[derive(Deserialize, Serialize)]
pub struct TestrunAssignmentWithTickets {
    #[serde(flatten)]
    pub assignment: TestrunAssignment,

    #[serde(default)]
    pub ticket_materials: Option<AttachedTicketMaterials>,
}

impl Debug for TestrunAssignmentWithTickets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        trait Attached {
            fn attached(&self) -> String;
        }

        impl<T> Attached for Option<T> {
            fn attached(&self) -> String {
                if self.is_some() {
                    "attached"
                } else {
                    "not attached"
                }
                .to_string()
            }
        }

        // no need to include full binary data behind the ticketbook data
        f.debug_struct("TestrunAssignmentWithTickets")
            .field("assignment", &self.assignment)
            .field("ticket_materials", &self.ticket_materials.attached())
            .finish()
    }
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
