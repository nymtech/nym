// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::LpClientError;
use nym_lp::session::{LpAction, LpInput};
use nym_lp_data::packet::frame::LpFrameKind;
use nym_lp_data::packet::{ForwardPacketData, LpFrame};
use rand010::RngExt;

pub(crate) fn convert_forward_data(request: ForwardPacketData) -> Result<LpInput, LpClientError> {
    let bytes = request.to_bytes();

    tracing::trace!(
        "Sending forward packet data request ({} bytes)",
        bytes.len()
    );

    Ok(LpInput::SendFrame(LpFrame::new_forward(bytes)))
}

pub(crate) fn try_convert_forward_response(action: LpAction) -> Result<Vec<u8>, LpClientError> {
    let response_data = match action {
        LpAction::DeliverFrame(data) => data,
        action => return Err(LpClientError::UnexpectedStateMachineAction { action }),
    };

    if response_data.kind() != LpFrameKind::Forward {
        return Err(LpClientError::UnexpectedLpPayload {
            typ: response_data.kind(),
        });
    }

    Ok(response_data.content.into())
}

pub async fn exponential_backoff_with_jitter(attempt: u32) {
    // Exponential backoff with jitter: 100ms, 200ms, 400ms, 800ms, 1600ms (capped)
    let base_delay_ms = 100u64 * (1 << attempt.min(4));
    let jitter_ms: u64 = rand010::rng().random_range(0..(base_delay_ms / 4 + 1));
    let delay = std::time::Duration::from_millis(base_delay_ms + jitter_ms);
    tracing::info!("Retrying registration after the following delay {delay:?}");
    tokio::time::sleep(delay).await;
}
