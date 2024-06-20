// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sphinx::addressing::Recipient;
use nym_wireguard_types::{GatewayClient, InitMessage};
use serde::{Deserialize, Serialize};

use crate::make_bincode_serializer;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthenticatorRequest {
    pub version: u8,
    pub data: AuthenticatorRequestData,
    pub reply_to: Recipient,
}

impl AuthenticatorRequest {
    pub fn from_reconstructed_message(
        message: &nym_sphinx::receiver::ReconstructedMessage,
    ) -> Result<Self, bincode::Error> {
        use bincode::Options;
        make_bincode_serializer().deserialize(&message.message)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuthenticatorRequestData {
    Initial(InitMessage),
    Final(GatewayClient),
}
