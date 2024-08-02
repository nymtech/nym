// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sphinx::addressing::Recipient;
use nym_wireguard_types::{GatewayClient, InitMessage, PeerPublicKey};
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
    pub version: u8,
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
                version: VERSION,
                data: AuthenticatorRequestData::Initial(init_message),
                reply_to,
                request_id,
            },
            request_id,
        )
    }

    pub fn new_final_request(gateway_client: GatewayClient, reply_to: Recipient) -> (Self, u64) {
        let request_id = generate_random();
        (
            Self {
                version: VERSION,
                data: AuthenticatorRequestData::Final(gateway_client),
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
                version: VERSION,
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
    Final(GatewayClient),
    QueryBandwidth(PeerPublicKey),
}
