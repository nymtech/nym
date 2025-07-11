// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::asymmetric::ed25519;
use nym_crypto::asymmetric::ed25519::serde_helpers::bs58_ed25519_signature;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// the trait is not public as it's only defined on types that are guaranteed to not panic when serialised
pub trait SignableMessageBody: Serialize + sealed::Sealed {
    fn sign(self, key: &ed25519::PrivateKey) -> SignedMessage<Self>
    where
        Self: Sized,
    {
        let signature = key.sign(self.plaintext());
        SignedMessage {
            body: self,
            signature,
        }
    }

    fn plaintext(&self) -> Vec<u8> {
        #[allow(clippy::unwrap_used)]
        // SAFETY: all types that implement this trait have valid serialisations
        serde_json::to_vec(&self).unwrap()
    }
}

impl<T> SignableMessageBody for T where T: Serialize + sealed::Sealed {}

#[derive(Clone, Serialize, Deserialize, Debug, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SignedMessage<T> {
    pub body: T,
    #[schema(value_type = String)]
    #[serde(with = "bs58_ed25519_signature")]
    pub signature: ed25519::Signature,
}

impl<T> SignedMessage<T> {
    pub fn verify_signature(&self, pub_key: &ed25519::PublicKey) -> bool
    where
        T: SignableMessageBody,
    {
        let plaintext = self.body.plaintext();
        if plaintext.is_empty() {
            return false;
        }

        pub_key.verify(&plaintext, &self.signature).is_ok()
    }
}

// make sure only our types can implement this trait (to ensure infallible serialisation)
pub(crate) mod sealed {
    use crate::ecash::models::*;
    use crate::models::{
        ChainBlocksStatusResponseBody, DetailedSignersStatusResponseBody, SignersStatusResponseBody,
    };

    pub trait Sealed {}

    // requests
    impl Sealed for IssuedTicketbooksChallengeCommitmentRequestBody {}
    impl Sealed for IssuedTicketbooksDataRequestBody {}

    // responses
    impl Sealed for IssuedTicketbooksChallengeCommitmentResponseBody {}
    impl Sealed for IssuedTicketbooksForResponseBody {}
    impl Sealed for IssuedTicketbooksDataResponseBody {}
    impl Sealed for EcashSignerStatusResponseBody {}
    impl Sealed for ChainBlocksStatusResponseBody {}
    impl Sealed for SignersStatusResponseBody {}
    impl Sealed for DetailedSignersStatusResponseBody {}
}
