// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_ip_packet_requests::response_helpers::IprResponseError;
use nym_sdk::mixnet::ReconstructedMessage;

use crate::{current::VERSION as CURRENT_VERSION, error::Result};

pub(crate) fn check_ipr_message_version(message: &ReconstructedMessage) -> Result<()> {
    nym_ip_packet_requests::response_helpers::check_ipr_message_version(
        &message.message,
        CURRENT_VERSION,
    )
    .map_err(|e| match e {
        IprResponseError::NoVersionByte => crate::Error::NoVersionInMessage,
        IprResponseError::VersionMismatch { expected, received } if received < expected => {
            crate::Error::ReceivedResponseWithOldVersion { expected, received }
        }
        IprResponseError::VersionMismatch { expected, received } => {
            crate::Error::ReceivedResponseWithNewVersion { expected, received }
        }
        _ => crate::Error::NoVersionInMessage,
    })
}
