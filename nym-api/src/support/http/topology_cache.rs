#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]


use std::collections::HashMap;
use nym_topology::NymTopology;
use serde::{Deserialize, Serialize};
use nym_mixnet_contract_common::{EpochId, Interval};
use std::time::Duration;
use time::OffsetDateTime;

pub struct Epoch {
	pub id: EpochId,
	pub current_epoch_start: OffsetDateTime,
    pub epoch_length: Duration,
}

impl From<Interval> for Epoch {
	fn from(value: Interval) -> Self {
		Self {
			id: value.current_epoch_id(),
			current_epoch_start: value.current_epoch_start(),
			epoch_length: value.epoch_length(),
		}
	}
}

/// Format for 
#[derive(Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema, utoipa::ToSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PayloadFormat {
    Json,
    BitCode,
}

pub(crate) struct TopologyCache {
	current_epoch: Epoch,
	formats: HashMap<PayloadFormat, SerializedTopology>,
	cached: NymTopology,
	hash: Option<Vec<u8>>,
	signature: Option<Vec<u8>>
}

pub(crate) struct SerializedTopology{
	bytes: Vec<u8>,
	signature: Vec<u8>,
}


impl TopologyCache {
	pub fn new(current_epoch: Epoch, initial: NymTopology) -> Self {
		Self {
			current_epoch,
			formats: HashMap::new(),
			cached: initial,
			hash: None,
			signature: None
		}
	}
}