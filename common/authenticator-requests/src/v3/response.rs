// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::registration::{RegistrationData, RegistredData, RemainingBandwidthData};
use nym_service_provider_requests_common::{Protocol, ServiceProviderType};
use nym_sphinx::addressing::Recipient;
use serde::{Deserialize, Serialize};

use crate::make_bincode_serializer;

use super::VERSION;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthenticatorResponse {
    pub protocol: Protocol,
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
            protocol: Protocol {
                service_provider_type: ServiceProviderType::Authenticator,
                version: VERSION,
            },
            data: AuthenticatorResponseData::PendingRegistration(PendingRegistrationResponse {
                reply: registration_data,
                reply_to,
                request_id,
            }),
            reply_to,
        }
    }

    pub fn new_registered(
        registred_data: RegistredData,
        reply_to: Recipient,
        request_id: u64,
    ) -> Self {
        Self {
            protocol: Protocol {
                service_provider_type: ServiceProviderType::Authenticator,
                version: VERSION,
            },
            data: AuthenticatorResponseData::Registered(RegisteredResponse {
                reply: registred_data,
                reply_to,
                request_id,
            }),
            reply_to,
        }
    }

    pub fn new_remaining_bandwidth(
        remaining_bandwidth_data: Option<RemainingBandwidthData>,
        reply_to: Recipient,
        request_id: u64,
    ) -> Self {
        Self {
            protocol: Protocol {
                service_provider_type: ServiceProviderType::Authenticator,
                version: VERSION,
            },
            data: AuthenticatorResponseData::RemainingBandwidth(RemainingBandwidthResponse {
                reply: remaining_bandwidth_data,
                reply_to,
                request_id,
            }),
            reply_to,
        }
    }

    pub fn new_topup_bandwidth(
        remaining_bandwidth_data: RemainingBandwidthData,
        reply_to: Recipient,
        request_id: u64,
    ) -> Self {
        Self {
            protocol: Protocol {
                service_provider_type: ServiceProviderType::Authenticator,
                version: VERSION,
            },
            data: AuthenticatorResponseData::TopUpBandwidth(TopUpBandwidthResponse {
                reply: remaining_bandwidth_data,
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
            AuthenticatorResponseData::RemainingBandwidth(response) => Some(response.request_id),
            AuthenticatorResponseData::TopUpBandwidth(response) => Some(response.request_id),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AuthenticatorResponseData {
    PendingRegistration(PendingRegistrationResponse),
    Registered(RegisteredResponse),
    RemainingBandwidth(RemainingBandwidthResponse),
    TopUpBandwidth(TopUpBandwidthResponse),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PendingRegistrationResponse {
    pub request_id: u64,
    pub reply_to: Recipient,
    pub reply: RegistrationData,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegisteredResponse {
    pub request_id: u64,
    pub reply_to: Recipient,
    pub reply: RegistredData,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RemainingBandwidthResponse {
    pub request_id: u64,
    pub reply_to: Recipient,
    pub reply: Option<RemainingBandwidthData>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TopUpBandwidthResponse {
    pub request_id: u64,
    pub reply_to: Recipient,
    pub reply: RemainingBandwidthData,
}
