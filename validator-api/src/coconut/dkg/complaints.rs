// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum ComplaintReason {
    MalformedBTEPublicKey,
    InvalidBTEPublicKey,
    MissingDealing,
    MalformedDealing,
    DealingVerificationError,
}
