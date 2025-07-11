// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_mixnet_contract_common::nym_node::Role;
use nym_mixnet_contract_common::{EpochId, KeyRotationId, KeyRotationState, NodeId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::time::Duration;
use time::OffsetDateTime;
use tracing::warn;
use utoipa::ToSchema;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct KeyRotationInfoResponse {
    #[serde(flatten)]
    pub details: KeyRotationDetails,

    // helper field that holds calculated data based on the `details` field
    // this is to expose the information in a format more easily accessible by humans
    // without having to do any calculations
    pub progress: KeyRotationProgressInfo,
}

impl From<KeyRotationDetails> for KeyRotationInfoResponse {
    fn from(details: KeyRotationDetails) -> Self {
        KeyRotationInfoResponse {
            details,
            progress: KeyRotationProgressInfo {
                current_key_rotation_id: details.current_key_rotation_id(),
                current_rotation_starting_epoch: details.current_rotation_starting_epoch_id(),
                current_rotation_ending_epoch: details.current_rotation_starting_epoch_id()
                    + details.key_rotation_state.validity_epochs
                    - 1,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct KeyRotationProgressInfo {
    pub current_key_rotation_id: u32,

    pub current_rotation_starting_epoch: u32,

    pub current_rotation_ending_epoch: u32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct KeyRotationDetails {
    pub key_rotation_state: KeyRotationState,

    #[schema(value_type = u32)]
    pub current_absolute_epoch_id: EpochId,

    #[serde(with = "time::serde::rfc3339")]
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub current_epoch_start: OffsetDateTime,

    pub epoch_duration: Duration,
}

impl KeyRotationDetails {
    pub fn current_key_rotation_id(&self) -> u32 {
        self.key_rotation_state
            .key_rotation_id(self.current_absolute_epoch_id)
    }

    pub fn next_rotation_starting_epoch_id(&self) -> EpochId {
        self.key_rotation_state
            .next_rotation_starting_epoch_id(self.current_absolute_epoch_id)
    }

    pub fn current_rotation_starting_epoch_id(&self) -> EpochId {
        self.key_rotation_state
            .current_rotation_starting_epoch_id(self.current_absolute_epoch_id)
    }

    fn current_epoch_progress(&self, now: OffsetDateTime) -> f32 {
        let elapsed = (now - self.current_epoch_start).as_seconds_f32();
        elapsed / self.epoch_duration.as_secs_f32()
    }

    pub fn is_epoch_stuck(&self) -> bool {
        let now = OffsetDateTime::now_utc();
        let progress = self.current_epoch_progress(now);
        if progress > 1. {
            let into_next = 1. - progress;
            // if epoch hasn't progressed for more than 20% of its duration, mark is as stuck
            if into_next > 0.2 {
                let diff_time =
                    Duration::from_secs_f32(into_next * self.epoch_duration.as_secs_f32());
                let expected_epoch_end = self.current_epoch_start + self.epoch_duration;
                warn!("the current epoch is expected to have been over by {expected_epoch_end}. it's already {} overdue!", humantime_serde::re::humantime::format_duration(diff_time));
                return true;
            }
        }

        false
    }

    // based on the current **TIME**, determine what's the expected current rotation id
    pub fn expected_current_rotation_id(&self) -> KeyRotationId {
        let now = OffsetDateTime::now_utc();
        let current_end = now + self.epoch_duration;
        if now < current_end {
            return self
                .key_rotation_state
                .key_rotation_id(self.current_absolute_epoch_id);
        }

        let diff = now - current_end;
        let passed_epochs = diff / self.epoch_duration;
        let expected_current_epoch = self.current_absolute_epoch_id + passed_epochs.floor() as u32;

        self.key_rotation_state
            .key_rotation_id(expected_current_epoch)
    }

    pub fn until_next_rotation(&self) -> Option<Duration> {
        let current_epoch_progress = self.current_epoch_progress(OffsetDateTime::now_utc());
        if current_epoch_progress > 1. {
            return None;
        }

        let next_rotation_epoch = self.next_rotation_starting_epoch_id();
        let full_remaining =
            (next_rotation_epoch - self.current_absolute_epoch_id).checked_add(1)?;

        let epochs_until_next_rotation = (1. - current_epoch_progress) + full_remaining as f32;

        Some(Duration::from_secs_f32(
            epochs_until_next_rotation * self.epoch_duration.as_secs_f32(),
        ))
    }

    pub fn epoch_start_time(&self, absolute_epoch_id: EpochId) -> OffsetDateTime {
        match absolute_epoch_id.cmp(&self.current_absolute_epoch_id) {
            Ordering::Less => {
                let diff = self.current_absolute_epoch_id - absolute_epoch_id;
                self.current_epoch_start - diff * self.epoch_duration
            }
            Ordering::Equal => self.current_epoch_start,
            Ordering::Greater => {
                let diff = absolute_epoch_id - self.current_absolute_epoch_id;
                self.current_epoch_start + diff * self.epoch_duration
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct RewardedSetResponse {
    #[serde(default)]
    #[schema(value_type = u32)]
    pub epoch_id: EpochId,

    pub entry_gateways: Vec<NodeId>,

    pub exit_gateways: Vec<NodeId>,

    pub layer1: Vec<NodeId>,

    pub layer2: Vec<NodeId>,

    pub layer3: Vec<NodeId>,

    pub standby: Vec<NodeId>,
}

impl From<RewardedSetResponse> for nym_mixnet_contract_common::EpochRewardedSet {
    fn from(res: RewardedSetResponse) -> Self {
        nym_mixnet_contract_common::EpochRewardedSet {
            epoch_id: res.epoch_id,
            assignment: nym_mixnet_contract_common::RewardedSet {
                entry_gateways: res.entry_gateways,
                exit_gateways: res.exit_gateways,
                layer1: res.layer1,
                layer2: res.layer2,
                layer3: res.layer3,
                standby: res.standby,
            },
        }
    }
}

impl From<nym_mixnet_contract_common::EpochRewardedSet> for RewardedSetResponse {
    fn from(r: nym_mixnet_contract_common::EpochRewardedSet) -> Self {
        RewardedSetResponse {
            epoch_id: r.epoch_id,
            entry_gateways: r.assignment.entry_gateways,
            exit_gateways: r.assignment.exit_gateways,
            layer1: r.assignment.layer1,
            layer2: r.assignment.layer2,
            layer3: r.assignment.layer3,
            standby: r.assignment.standby,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, JsonSchema, ToSchema)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export, export_to = "ts-packages/types/src/types/rust/DisplayRole.ts")
)]
pub enum DisplayRole {
    EntryGateway,
    Layer1,
    Layer2,
    Layer3,
    ExitGateway,
    Standby,
}

impl From<Role> for DisplayRole {
    fn from(role: Role) -> Self {
        match role {
            Role::EntryGateway => DisplayRole::EntryGateway,
            Role::Layer1 => DisplayRole::Layer1,
            Role::Layer2 => DisplayRole::Layer2,
            Role::Layer3 => DisplayRole::Layer3,
            Role::ExitGateway => DisplayRole::ExitGateway,
            Role::Standby => DisplayRole::Standby,
        }
    }
}

impl From<DisplayRole> for Role {
    fn from(role: DisplayRole) -> Self {
        match role {
            DisplayRole::EntryGateway => Role::EntryGateway,
            DisplayRole::Layer1 => Role::Layer1,
            DisplayRole::Layer2 => Role::Layer2,
            DisplayRole::Layer3 => Role::Layer3,
            DisplayRole::ExitGateway => Role::ExitGateway,
            DisplayRole::Standby => Role::Standby,
        }
    }
}
