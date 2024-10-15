// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::{
    registration::{FinalMessage, InitMessage},
    topup::TopUpMessage,
};
use nym_service_provider_requests_common::{Protocol, ServiceProviderType};
use nym_sphinx::addressing::Recipient;
use nym_wireguard_types::PeerPublicKey;
use serde::{Deserialize, Serialize};

use crate::make_bincode_serializer;

use super::VERSION;

fn generate_random() -> u64 {
    use rand::RngCore;
    let mut rng = rand::rngs::OsRng;
    rng.next_u64()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthenticatorRequest {
    pub protocol: Protocol,
    pub data: AuthenticatorRequestData,
    pub reply_to: Recipient,
    pub request_id: u64,
}

impl AuthenticatorRequest {
    pub fn from_reconstructed_message(
        message: &nym_sphinx::receiver::ReconstructedMessage,
    ) -> Result<Self, bincode::Error> {
        use bincode::Options;
        make_bincode_serializer().deserialize(&message.message)
    }

    pub fn new_initial_request(init_message: InitMessage, reply_to: Recipient) -> (Self, u64) {
        let request_id = generate_random();
        (
            Self {
                protocol: Protocol {
                    service_provider_type: ServiceProviderType::Authenticator,
                    version: VERSION,
                },
                data: AuthenticatorRequestData::Initial(init_message),
                reply_to,
                request_id,
            },
            request_id,
        )
    }

    pub fn new_final_request(final_message: FinalMessage, reply_to: Recipient) -> (Self, u64) {
        let request_id = generate_random();
        (
            Self {
                protocol: Protocol {
                    service_provider_type: ServiceProviderType::Authenticator,
                    version: VERSION,
                },
                data: AuthenticatorRequestData::Final(Box::new(final_message)),
                reply_to,
                request_id,
            },
            request_id,
        )
    }

    pub fn new_query_request(peer_public_key: PeerPublicKey, reply_to: Recipient) -> (Self, u64) {
        let request_id = generate_random();
        (
            Self {
                protocol: Protocol {
                    service_provider_type: ServiceProviderType::Authenticator,
                    version: VERSION,
                },
                data: AuthenticatorRequestData::QueryBandwidth(peer_public_key),
                reply_to,
                request_id,
            },
            request_id,
        )
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        use bincode::Options;
        make_bincode_serializer().serialize(self)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuthenticatorRequestData {
    Initial(InitMessage),
    Final(Box<FinalMessage>),
    QueryBandwidth(PeerPublicKey),
    TopUpBandwidth(TopUpMessage),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn check_first_bytes_protocol() {
        let version = 2;
        let data = AuthenticatorRequest {
            protocol: Protocol { version, service_provider_type: ServiceProviderType::Authenticator },
            data: AuthenticatorRequestData::Initial(InitMessage::new(
                PeerPublicKey::from_str("yvNUDpT5l7W/xDhiu6HkqTHDQwbs/B3J5UrLmORl1EQ=").unwrap(),
            )),
            reply_to: Recipient::try_from_base58_string("D1rrpsysCGCYXy9saP8y3kmNpGtJZUXN9SvFoUcqAsM9.9Ssso1ea5NfkbMASdiseDSjTN1fSWda5SgEVjdSN4CvV@GJqd3ZxpXWSNxTfx7B1pPtswpetH4LnJdFeLeuY5KUuN").unwrap(),
            request_id: 1,
        };
        let bytes = *data.to_bytes().unwrap().first_chunk::<2>().unwrap();
        assert_eq!(bytes, [version, ServiceProviderType::Authenticator as u8]);
    }
}
