use serde::{Deserialize, Serialize};

pub(crate) struct GatewayRecord {
    pub(crate) identity_key: String,
    pub(crate) bonded: bool,
    pub(crate) blacklisted: bool,
    pub(crate) self_described: Option<String>,
    pub(crate) explorer_pretty_bond: Option<String>,
    pub(crate) last_updated_utc: i64,
    pub(crate) performance: u8,
}

pub(crate) struct MixnodeRecord {
    pub(crate) mix_id: u32,
    pub(crate) identity_key: String,
    pub(crate) bonded: bool,
    pub(crate) total_stake: i64,
    pub(crate) host: String,
    pub(crate) http_port: u16,
    pub(crate) blacklisted: bool,
    pub(crate) full_details: String,
    pub(crate) self_described: Option<String>,
    pub(crate) last_updated_utc: i64,
    pub(crate) is_dp_delegatee: bool,
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub(crate) struct BondedStatusDto {
    pub(crate) id: i64,
    pub(crate) identity_key: String,
    pub(crate) bonded: bool,
}

#[allow(unused)]
#[derive(Debug, Clone, Default)]
pub(crate) struct SummaryDto {
    pub key: String,
    pub value_json: String,
    pub last_updated_utc: i64,
}

pub const MIXNODES_BONDED_COUNT: &str = "mixnodes.bonded.count";
pub const MIXNODES_BONDED_ACTIVE: &str = "mixnodes.bonded.active";
pub const MIXNODES_BONDED_INACTIVE: &str = "mixnodes.bonded.inactive";
pub const MIXNODES_BONDED_RESERVE: &str = "mixnodes.bonded.reserve";
pub const MIXNODES_BLACKLISTED_COUNT: &str = "mixnodes.blacklisted.count";

pub const GATEWAYS_BONDED_COUNT: &str = "gateways.bonded.count";
pub const GATEWAYS_EXPLORER_COUNT: &str = "gateways.explorer.count";
pub const GATEWAYS_BLACKLISTED_COUNT: &str = "gateways.blacklisted.count";

pub const MIXNODES_HISTORICAL_COUNT: &str = "mixnodes.historical.count";
pub const GATEWAYS_HISTORICAL_COUNT: &str = "gateways.historical.count";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct NetworkSummary {
    pub(crate) mixnodes: mixnode::MixnodeSummary,
    pub(crate) gateways: gateway::GatewaySummary,
}

mod mixnode {
    use super::*;

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct MixnodeSummary {
        pub bonded: MixnodeSummaryBonded,
        pub blacklisted: MixnodeSummaryBlacklisted,
        pub historical: MixnodeSummaryHistorical,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct MixnodeSummaryBonded {
        pub count: i32,
        pub active: i32,
        pub inactive: i32,
        pub reserve: i32,
        pub last_updated_utc: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct MixnodeSummaryBlacklisted {
        pub count: i32,
        pub last_updated_utc: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct MixnodeSummaryHistorical {
        pub count: i32,
        pub last_updated_utc: String,
    }
}

mod gateway {
    use super::*;

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct GatewaySummary {
        pub bonded: GatewaySummaryBonded,
        pub blacklisted: GatewaySummaryBlacklisted,
        pub historical: GatewaySummaryHistorical,
        pub explorer: GatewaySummaryExplorer,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct GatewaySummaryExplorer {
        pub count: i32,
        pub last_updated_utc: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct GatewaySummaryBonded {
        pub count: i32,
        pub last_updated_utc: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct GatewaySummaryHistorical {
        pub count: i32,
        pub last_updated_utc: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct GatewaySummaryBlacklisted {
        pub count: i32,
        pub last_updated_utc: String,
    }
}
