// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_vpn_api_client::types::ScoreThresholds;

pub(crate) const HIGH_SCORE_THRESHOLD: u8 = 80;
pub(crate) const MEDIUM_SCORE_THRESHOLD: u8 = 60;
pub(crate) const LOW_SCORE_THRESHOLD: u8 = 0;

#[derive(Clone)]
pub enum Score {
    High(u8),
    Medium(u8),
    Low(u8),
    None,
}

impl Score {
    pub fn update_to_new_thresholds(&mut self, thresholds: ScoreThresholds) {
        let score = match self {
            Score::None => return,
            Score::High(score) | Score::Medium(score) | Score::Low(score) => *score,
        };
        *self = if score > thresholds.high {
            Score::High(score)
        } else if score > thresholds.medium {
            Score::Medium(score)
        } else if score > thresholds.low {
            Score::Low(score)
        } else {
            Score::None
        };
    }
}
