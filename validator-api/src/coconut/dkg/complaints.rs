// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ComplaintReason {
    MalformedBTEPublicKey,
    InvalidBTEPublicKey,
    MissingDealing,
    MalformedDealing,
    DealingVerificationError,
}
