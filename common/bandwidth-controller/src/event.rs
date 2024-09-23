// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// See other comments for other TaskStatus message enumds about abusing the Error trait when we
// should have a new trait for TaskStatus messages
#[derive(Debug, thiserror::Error)]
pub enum BandwidthStatusMessage {
    #[error("remaining bandwidth: {0}")]
    RemainingBandwidth(i64),

    #[error("no bandwidth left")]
    NoBandwidth,
}

impl nym_task::manager::TaskStatusEvent for BandwidthStatusMessage {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
