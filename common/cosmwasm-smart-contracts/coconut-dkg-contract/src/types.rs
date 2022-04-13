// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Blacklisting {
    reason: BlacklistingReason,
    height: u64,
}

impl Display for Blacklisting {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "blacklisted at block height {}. reason given: {}",
            self.height, self.height
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BlacklistingReason {
    InactiveForConsecutiveEpochs,
}

impl Display for BlacklistingReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BlacklistingReason::InactiveForConsecutiveEpochs => {
                write!(f, "has been inactive for multiple consecutive epochs")
            }
        }
    }
}
