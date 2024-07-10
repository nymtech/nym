// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sphinx::addressing::Recipient;
use nym_wireguard_types::registration::RegistrationData;
use serde::{Deserialize, Serialize};

use crate::make_bincode_serializer;

use super::VERSION;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthenticatorResponse {
    pub version: u8,
    pub data: AuthenticatorResponseData,
    pub reply_to: Recipient,
}

impl AuthenticatorResponse {
    pub fn new_pending_registration_success(
        registration_data: RegistrationData,
        request_id: u64,
        reply_to: Recipient,
    ) -> Self {
        Self {
            version: VERSION,
            data: AuthenticatorResponseData::PendingRegistration(PendingRegistrationResponse {
                reply: registration_data,
                reply_to,
                request_id,
            }),
            reply_to,
        }
    }

    pub fn new_registered(reply_to: Recipient, request_id: u64) -> Self {
        Self {
            version: VERSION,
            data: AuthenticatorResponseData::Registered(RegisteredResponse {
                reply_to,
                request_id,
            }),
            reply_to,
        }
    }

    pub fn recipient(&self) -> Recipient {
        self.reply_to
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        use bincode::Options;
        make_bincode_serializer().serialize(self)
    }

    pub fn from_reconstructed_message(
        message: &nym_sphinx::receiver::ReconstructedMessage,
    ) -> Result<Self, bincode::Error> {
        use bincode::Options;
        make_bincode_serializer().deserialize(&message.message)
    }

    pub fn id(&self) -> Option<u64> {
        match &self.data {
            AuthenticatorResponseData::PendingRegistration(response) => Some(response.request_id),
            AuthenticatorResponseData::Registered(response) => Some(response.request_id),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuthenticatorResponseData {
    PendingRegistration(PendingRegistrationResponse),
    Registered(RegisteredResponse),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingRegistrationResponse {
    pub request_id: u64,
    pub reply_to: Recipient,
    pub reply: RegistrationData,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisteredResponse {
    pub request_id: u64,
    pub reply_to: Recipient,
}
