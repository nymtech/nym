// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials_interface::{
    BandwidthCredential, CredentialSpendingData, TicketType, UnknownTicketType,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum CurrentUpgradeModeStatus {
    Enabled,
    Disabled,
    // everything pre-v6
    Unknown,
}

impl CurrentUpgradeModeStatus {
    pub fn is_enabled(&self) -> bool {
        matches!(self, CurrentUpgradeModeStatus::Enabled)
    }
}

impl From<bool> for CurrentUpgradeModeStatus {
    fn from(value: bool) -> Self {
        if value {
            CurrentUpgradeModeStatus::Enabled
        } else {
            CurrentUpgradeModeStatus::Disabled
        }
    }
}

impl From<CurrentUpgradeModeStatus> for Option<bool> {
    fn from(value: CurrentUpgradeModeStatus) -> Self {
        match value {
            CurrentUpgradeModeStatus::Enabled => Some(true),
            CurrentUpgradeModeStatus::Disabled => Some(false),
            CurrentUpgradeModeStatus::Unknown => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct BandwidthClaim {
    pub credential: BandwidthCredential,
    pub kind: TicketType,
}

impl TryFrom<CredentialSpendingData> for BandwidthClaim {
    type Error = UnknownTicketType;

    fn try_from(credential: CredentialSpendingData) -> Result<Self, Self::Error> {
        Ok(BandwidthClaim {
            kind: TicketType::try_from_encoded(credential.payment.t_type)?,
            credential: BandwidthCredential::from(credential),
        })
    }
}
