// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use nym_ip_packet_requests::response_helpers::MixnetMessageOutcome;

use nym_ip_packet_requests::response_helpers;

use crate::ip_packet_client::current::VERSION as CURRENT_VERSION;

/// Check that the first byte of an IPR message matches the expected protocol version.
pub(crate) fn check_ipr_message_version(data: &[u8]) -> Result<(), crate::Error> {
    response_helpers::check_ipr_message_version(data, CURRENT_VERSION)
        .map_err(|e| crate::Error::IPRMessageVersionCheckFailed(e.to_string()))
}

/// Parse raw IPR response bytes into an outcome.
pub fn handle_ipr_response(data: &[u8]) -> Result<Option<MixnetMessageOutcome>, crate::Error> {
    check_ipr_message_version(data)?;
    Ok(response_helpers::handle_ipr_response(data))
}
