// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use dkg::error::DkgError;

pub(crate) enum ComplaintReason {
    MalformedBTEPublicKey,
    MissingDealing,
    MalformedDealing(DkgError),
    DealingVerificationError(DkgError),
}
