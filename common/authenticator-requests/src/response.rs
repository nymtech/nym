// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::traits::{
    Id, PendingRegistrationResponse, RegisteredResponse, RemainingBandwidthResponse,
    TopUpBandwidthResponse,
};
use crate::{v2, v3, v4, v5};

#[derive(Debug)]
pub enum AuthenticatorResponse {
    PendingRegistration(Box<dyn PendingRegistrationResponse + Send + Sync + 'static>),
    Registered(Box<dyn RegisteredResponse + Send + Sync + 'static>),
    RemainingBandwidth(Box<dyn RemainingBandwidthResponse + Send + Sync + 'static>),
    TopUpBandwidth(Box<dyn TopUpBandwidthResponse + Send + Sync + 'static>),
}

impl Id for AuthenticatorResponse {
    fn id(&self) -> u64 {
        match self {
            AuthenticatorResponse::PendingRegistration(pending_registration_response) => {
                pending_registration_response.id()
            }
            AuthenticatorResponse::Registered(registered_response) => registered_response.id(),
            AuthenticatorResponse::RemainingBandwidth(remaining_bandwidth_response) => {
                remaining_bandwidth_response.id()
            }
            AuthenticatorResponse::TopUpBandwidth(top_up_bandwidth_response) => {
                top_up_bandwidth_response.id()
            }
        }
    }
}

impl From<v2::response::AuthenticatorResponse> for AuthenticatorResponse {
    fn from(value: v2::response::AuthenticatorResponse) -> Self {
        match value.data {
            v2::response::AuthenticatorResponseData::PendingRegistration(
                pending_registration_response,
            ) => Self::PendingRegistration(Box::new(pending_registration_response)),
            v2::response::AuthenticatorResponseData::Registered(registered_response) => {
                Self::Registered(Box::new(registered_response))
            }
            v2::response::AuthenticatorResponseData::RemainingBandwidth(
                remaining_bandwidth_response,
            ) => Self::RemainingBandwidth(Box::new(remaining_bandwidth_response)),
        }
    }
}

impl From<v3::response::AuthenticatorResponse> for AuthenticatorResponse {
    fn from(value: v3::response::AuthenticatorResponse) -> Self {
        match value.data {
            v3::response::AuthenticatorResponseData::PendingRegistration(
                pending_registration_response,
            ) => Self::PendingRegistration(Box::new(pending_registration_response)),
            v3::response::AuthenticatorResponseData::Registered(registered_response) => {
                Self::Registered(Box::new(registered_response))
            }
            v3::response::AuthenticatorResponseData::RemainingBandwidth(
                remaining_bandwidth_response,
            ) => Self::RemainingBandwidth(Box::new(remaining_bandwidth_response)),
            v3::response::AuthenticatorResponseData::TopUpBandwidth(top_up_bandwidth_response) => {
                Self::TopUpBandwidth(Box::new(top_up_bandwidth_response))
            }
        }
    }
}

impl From<v4::response::AuthenticatorResponse> for AuthenticatorResponse {
    fn from(value: v4::response::AuthenticatorResponse) -> Self {
        match value.data {
            v4::response::AuthenticatorResponseData::PendingRegistration(
                pending_registration_response,
            ) => Self::PendingRegistration(Box::new(pending_registration_response)),
            v4::response::AuthenticatorResponseData::Registered(registered_response) => {
                Self::Registered(Box::new(registered_response))
            }
            v4::response::AuthenticatorResponseData::RemainingBandwidth(
                remaining_bandwidth_response,
            ) => Self::RemainingBandwidth(Box::new(remaining_bandwidth_response)),
            v4::response::AuthenticatorResponseData::TopUpBandwidth(top_up_bandwidth_response) => {
                Self::TopUpBandwidth(Box::new(top_up_bandwidth_response))
            }
        }
    }
}

impl From<v5::response::AuthenticatorResponse> for AuthenticatorResponse {
    fn from(value: v5::response::AuthenticatorResponse) -> Self {
        match value.data {
            v5::response::AuthenticatorResponseData::PendingRegistration(
                pending_registration_response,
            ) => Self::PendingRegistration(Box::new(pending_registration_response)),
            v5::response::AuthenticatorResponseData::Registered(registered_response) => {
                Self::Registered(Box::new(registered_response))
            }
            v5::response::AuthenticatorResponseData::RemainingBandwidth(
                remaining_bandwidth_response,
            ) => Self::RemainingBandwidth(Box::new(remaining_bandwidth_response)),
            v5::response::AuthenticatorResponseData::TopUpBandwidth(top_up_bandwidth_response) => {
                Self::TopUpBandwidth(Box::new(top_up_bandwidth_response))
            }
        }
    }
}
