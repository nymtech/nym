// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::cmp::Ordering;

use nym_sdk::mixnet::ReconstructedMessage;

use crate::{current::VERSION as CURRENT_VERSION, error::Result, Error};

pub(crate) fn check_ipr_message_version(message: &ReconstructedMessage) -> Result<()> {
    // Assuming it's a IPR message, it will have a version as its first byte
    if let Some(version) = message.message.first() {
        match version.cmp(&CURRENT_VERSION) {
            Ordering::Greater => Err(Error::ReceivedResponseWithNewVersion {
                expected: CURRENT_VERSION,
                received: *version,
            }),
            Ordering::Less => Err(Error::ReceivedResponseWithOldVersion {
                expected: CURRENT_VERSION,
                received: *version,
            }),
            Ordering::Equal => {
                // We're good
                Ok(())
            }
        }
    } else {
        Err(Error::NoVersionInMessage)
    }
}
