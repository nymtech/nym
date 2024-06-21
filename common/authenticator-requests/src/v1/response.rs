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
        reply_to: Recipient,
    ) -> Self {
        Self {
            version: VERSION,
            data: AuthenticatorResponseData::PendingRegistration(registration_data),
            reply_to,
        }
    }

    pub fn new_registered(reply_to: Recipient) -> Self {
        Self {
            version: VERSION,
            data: AuthenticatorResponseData::Registered,
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuthenticatorResponseData {
    PendingRegistration(RegistrationData),
    Registered,
}
