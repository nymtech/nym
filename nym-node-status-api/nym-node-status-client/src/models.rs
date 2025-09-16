pub use nym_credentials::ecash::bandwidth::serialiser::{VersionSerialised, VersionedSerialise};
use nym_credentials::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures, EpochVerificationKey,
    Error, IssuedTicketBook,
};
use nym_crypto::asymmetric::ed25519::{PublicKey, Signature};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::error;

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
    pub coin_indices_signatures: Vec<VersionSerialised<AggregatedCoinIndicesSignatures>>,

    pub expiration_date_signatures: Vec<VersionSerialised<AggregatedExpirationDateSignatures>>,

    pub master_verification_keys: Vec<VersionSerialised<EpochVerificationKey>>,

    // we need one ticket per type
    pub attached_tickets: Vec<AttachedTicket>,
}

impl AttachedTicketMaterials {
    pub fn to_serialised_string(&self) -> String {
        // TODO: we're losing revision here, but given we control both ends of the pipeline,
        // that's fine. we can just pass it as a separate argument
        let serialised = self.pack();
        bs58::encode(serialised.data).into_string()
    }

    pub fn from_serialised_string(raw: String, revision: u8) -> Result<Self, Error> {
        let bytes = bs58::decode(raw)
            .into_vec()
            .inspect_err(|err| error!("malformed bytes encoding: {err}"))
            .unwrap_or_default();
        Self::try_unpack(&bytes, revision)
    }
}

impl VersionedSerialise for AttachedTicketMaterials {
    const CURRENT_SERIALISATION_REVISION: u8 = 1;

    fn try_unpack(b: &[u8], revision: impl Into<Option<u8>>) -> Result<Self, Error>
    where
        Self: DeserializeOwned,
    {
        let revision = revision
            .into()
            .unwrap_or(<Self as VersionedSerialise>::CURRENT_SERIALISATION_REVISION);

        match revision {
            1 => Self::try_unpack_current(b),
            _ => Err(Error::UnknownSerializationRevision { revision }),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TestrunAssignment {
    pub testrun_id: i32,
    pub assigned_at_utc: i64,
    pub gateway_identity_key: String,
}

impl TestrunAssignment {
    pub fn with_ticket_materials(
        self,
        materials: AttachedTicketMaterials,
    ) -> TestrunAssignmentWithTickets {
        TestrunAssignmentWithTickets {
            assignment: self,
            ticket_materials: Some(materials),
        }
    }

    pub fn with_no_ticket_materials(self) -> TestrunAssignmentWithTickets {
        TestrunAssignmentWithTickets {
            assignment: self,
            ticket_materials: None,
        }
    }
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
