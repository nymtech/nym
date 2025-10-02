// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sdk::mixnet::{IncludedSurbs, Recipient, TransmissionLane};

pub(crate) fn create_input_message(
    recipient: Recipient,
    data: Vec<u8>,
    surbs: IncludedSurbs,
) -> nym_sdk::mixnet::InputMessage {
    match surbs {
        IncludedSurbs::Amount(surbs) => nym_sdk::mixnet::InputMessage::new_anonymous(
            recipient,
            data,
            surbs,
            TransmissionLane::General,
            None,
        ),
        IncludedSurbs::ExposeSelfAddress => nym_sdk::mixnet::InputMessage::new_regular(
            recipient,
            data,
            TransmissionLane::General,
            None,
        ),
    }
}
