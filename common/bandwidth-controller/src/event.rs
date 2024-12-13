// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug)]
pub enum BandwidthStatusMessage {
    RemainingBandwidth(i64),
    NoBandwidth,
}

impl std::fmt::Display for BandwidthStatusMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BandwidthStatusMessage::RemainingBandwidth(b) => {
                write!(f, "remaining bandwidth: {}", b)
            }
            BandwidthStatusMessage::NoBandwidth => write!(f, "no bandwidth left"),
        }
    }
}

impl nym_task::TaskStatusEvent for BandwidthStatusMessage {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
