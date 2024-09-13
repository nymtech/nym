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
    pub(crate) key: String,
    pub(crate) value_json: String,
    pub(crate) last_updated_utc: i64,
}

pub(crate) const MIXNODES_BONDED_COUNT: &str = "mixnodes.bonded.count";
pub(crate) const MIXNODES_BONDED_ACTIVE: &str = "mixnodes.bonded.active";
pub(crate) const MIXNODES_BONDED_INACTIVE: &str = "mixnodes.bonded.inactive";
pub(crate) const MIXNODES_BONDED_RESERVE: &str = "mixnodes.bonded.reserve";
pub(crate) const MIXNODES_BLACKLISTED_COUNT: &str = "mixnodes.blacklisted.count";

pub(crate) const GATEWAYS_BONDED_COUNT: &str = "gateways.bonded.count";
pub(crate) const GATEWAYS_EXPLORER_COUNT: &str = "gateways.explorer.count";
pub(crate) const GATEWAYS_BLACKLISTED_COUNT: &str = "gateways.blacklisted.count";

pub(crate) const MIXNODES_HISTORICAL_COUNT: &str = "mixnodes.historical.count";
pub(crate) const GATEWAYS_HISTORICAL_COUNT: &str = "gateways.historical.count";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct NetworkSummary {
    pub(crate) mixnodes: mixnode::MixnodeSummary,
    pub(crate) gateways: gateway::GatewaySummary,
}

pub(crate) mod mixnode {
    use super::*;

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub(crate) struct MixnodeSummary {
        pub(crate) bonded: MixnodeSummaryBonded,
        pub(crate) blacklisted: MixnodeSummaryBlacklisted,
        pub(crate) historical: MixnodeSummaryHistorical,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub(crate) struct MixnodeSummaryBonded {
        pub(crate) count: i32,
        pub(crate) active: i32,
        pub(crate) inactive: i32,
        pub(crate) reserve: i32,
        pub(crate) last_updated_utc: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub(crate) struct MixnodeSummaryBlacklisted {
        pub(crate) count: i32,
        pub(crate) last_updated_utc: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub(crate) struct MixnodeSummaryHistorical {
        pub(crate) count: i32,
        pub(crate) last_updated_utc: String,
    }
}

pub(crate) mod gateway {
    use super::*;

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub(crate) struct GatewaySummary {
        pub(crate) bonded: GatewaySummaryBonded,
        pub(crate) blacklisted: GatewaySummaryBlacklisted,
        pub(crate) historical: GatewaySummaryHistorical,
        pub(crate) explorer: GatewaySummaryExplorer,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub(crate) struct GatewaySummaryExplorer {
        pub(crate) count: i32,
        pub(crate) last_updated_utc: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub(crate) struct GatewaySummaryBonded {
        pub(crate) count: i32,
        pub(crate) last_updated_utc: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub(crate) struct GatewaySummaryHistorical {
        pub(crate) count: i32,
        pub(crate) last_updated_utc: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub(crate) struct GatewaySummaryBlacklisted {
        pub(crate) count: i32,
        pub(crate) last_updated_utc: String,
    }
}
