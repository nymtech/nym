// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum ComplaintReason {
    MalformedBTEPublicKey,
    InvalidBTEPublicKey,
    MissingDealing,
    MalformedDealing,
    DealingVerificationError,
}

impl Display for ComplaintReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ComplaintReason::MalformedBTEPublicKey => {
                write!(f, "provided BTE Public key is malformed")
            }
            ComplaintReason::InvalidBTEPublicKey => {
                write!(f, "provided BTE public key does not verify correctly")
            }
            ComplaintReason::MissingDealing => write!(f, "one of the dealings is missing"),
            ComplaintReason::MalformedDealing => {
                write!(f, "one of the provided dealings is malformed")
            }
            ComplaintReason::DealingVerificationError => {
                write!(f, "failed to verify one of the dealings")
            }
        }
    }
}
