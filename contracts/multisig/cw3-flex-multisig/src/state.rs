use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw4::Cw4Contract;
use cw_storage_plus::Item;
use cw_utils::{Duration, Threshold};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub threshold: Threshold,
    pub max_voting_period: Duration,
    // Total weight and voters are queried from this contract
    pub group_addr: Cw4Contract,
}

// unique items
pub const CONFIG: Item<Config> = Item::new("config");
