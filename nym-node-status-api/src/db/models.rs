use crate::{
    http::{self, models::SummaryHistory},
    monitor::NumericalCheckedCast,
};
use nym_node_requests::api::v1::node::models::NodeDescription;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub(crate) struct GatewayRecord {
    pub(crate) identity_key: String,
    pub(crate) bonded: bool,
    pub(crate) blacklisted: bool,
    pub(crate) self_described: Option<String>,
    pub(crate) explorer_pretty_bond: Option<String>,
    pub(crate) last_updated_utc: i64,
    pub(crate) performance: u8,
}

#[derive(Debug, Clone)]
pub(crate) struct GatewayDto {
    pub(crate) gateway_identity_key: String,
    pub(crate) bonded: bool,
    pub(crate) blacklisted: bool,
    pub(crate) performance: i64,
    pub(crate) self_described: Option<String>,
    pub(crate) explorer_pretty_bond: Option<String>,
    pub(crate) last_probe_result: Option<String>,
    pub(crate) last_probe_log: Option<String>,
    pub(crate) last_testrun_utc: Option<i64>,
    pub(crate) last_updated_utc: i64,
    pub(crate) moniker: String,
    pub(crate) security_contact: String,
    pub(crate) details: String,
    pub(crate) website: String,
}

impl TryFrom<GatewayDto> for http::models::Gateway {
    type Error = anyhow::Error;

    fn try_from(value: GatewayDto) -> Result<Self, Self::Error> {
        // Instead of using routing_score_successes / routing_score_samples, we use the
        // number of successful testruns in the last 24h.
        let routing_score = 0f32;
        let config_score = 0u32;
        let last_updated_utc =
            timestamp_as_utc(value.last_updated_utc.cast_checked()?).to_rfc3339();
        let last_testrun_utc = value
            .last_testrun_utc
            .and_then(|i| i.cast_checked().ok())
            .map(|t| timestamp_as_utc(t).to_rfc3339());

        let self_described = value.self_described.clone().unwrap_or("null".to_string());
        let explorer_pretty_bond = value
            .explorer_pretty_bond
            .clone()
            .unwrap_or("null".to_string());
        let last_probe_result = value
            .last_probe_result
            .clone()
            .unwrap_or("null".to_string());
        let last_probe_log = value.last_probe_log.clone();

        let self_described = serde_json::from_str(&self_described).unwrap_or(None);
        let explorer_pretty_bond = serde_json::from_str(&explorer_pretty_bond).unwrap_or(None);
        let last_probe_result = serde_json::from_str(&last_probe_result).unwrap_or(None);

        let bonded = value.bonded;
        let blacklisted = value.blacklisted;
        let performance = value.performance as u8;

        let description = NodeDescription {
            moniker: value.moniker.clone(),
            website: value.website.clone(),
            security_contact: value.security_contact.clone(),
            details: value.details.clone(),
        };

        Ok(http::models::Gateway {
            gateway_identity_key: value.gateway_identity_key.clone(),
            bonded,
            blacklisted,
            performance,
            self_described,
            explorer_pretty_bond,
            description,
            last_probe_result,
            last_probe_log,
            routing_score,
            config_score,
            last_testrun_utc,
            last_updated_utc,
        })
    }
}

fn timestamp_as_utc(unix_timestamp: u64) -> chrono::DateTime<chrono::Utc> {
    let d = std::time::UNIX_EPOCH + std::time::Duration::from_secs(unix_timestamp);
    d.into()
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

#[derive(Debug, Clone)]
pub(crate) struct MixnodeDto {
    pub(crate) mix_id: i64,
    pub(crate) bonded: bool,
    pub(crate) blacklisted: bool,
    pub(crate) is_dp_delegatee: bool,
    pub(crate) total_stake: i64,
    pub(crate) full_details: String,
    pub(crate) self_described: Option<String>,
    pub(crate) last_updated_utc: i64,
    pub(crate) moniker: String,
    pub(crate) website: String,
    pub(crate) security_contact: String,
    pub(crate) details: String,
}

impl TryFrom<MixnodeDto> for http::models::Mixnode {
    type Error = anyhow::Error;

    fn try_from(value: MixnodeDto) -> Result<Self, Self::Error> {
        let mix_id = value.mix_id.cast_checked()?;
        let full_details = value.full_details.clone();
        let full_details = serde_json::from_str(&full_details).unwrap_or(None);

        let self_described = value
            .self_described
            .clone()
            .map(|v| serde_json::from_str(&v).unwrap_or(serde_json::Value::Null));

        let last_updated_utc =
            timestamp_as_utc(value.last_updated_utc.cast_checked()?).to_rfc3339();
        let blacklisted = value.blacklisted;
        let is_dp_delegatee = value.is_dp_delegatee;
        let moniker = value.moniker.clone();
        let website = value.website.clone();
        let security_contact = value.security_contact.clone();
        let details = value.details.clone();

        Ok(http::models::Mixnode {
            mix_id,
            bonded: value.bonded,
            blacklisted,
            is_dp_delegatee,
            total_stake: value.total_stake,
            full_details,
            description: NodeDescription {
                moniker,
                website,
                security_contact,
                details,
            },
            self_described,
            last_updated_utc,
        })
    }
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

#[derive(Debug, Clone, Default)]
pub(crate) struct SummaryHistoryDto {
    #[allow(dead_code)]
    pub id: i64,
    pub date: String,
    pub value_json: String,
    pub timestamp_utc: i64,
}

impl TryFrom<SummaryHistoryDto> for SummaryHistory {
    type Error = anyhow::Error;

    fn try_from(value: SummaryHistoryDto) -> Result<Self, Self::Error> {
        let value_json = serde_json::from_str(&value.value_json).unwrap_or_default();
        Ok(SummaryHistory {
            value_json,
            date: value.date.clone(),
            timestamp_utc: timestamp_as_utc(value.timestamp_utc.cast_checked()?).to_rfc3339(),
        })
    }
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

// `utoipa`` goes crazy if you use module-qualified prefix as field type so we
//  have to import it
use gateway::GatewaySummary;
use mixnode::MixnodeSummary;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub(crate) struct NetworkSummary {
    pub(crate) mixnodes: MixnodeSummary,
    pub(crate) gateways: GatewaySummary,
}

pub(crate) mod mixnode {
    use super::*;

    #[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
    pub(crate) struct MixnodeSummary {
        pub(crate) bonded: MixnodeSummaryBonded,
        pub(crate) blacklisted: MixnodeSummaryBlacklisted,
        pub(crate) historical: MixnodeSummaryHistorical,
    }

    #[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
    pub(crate) struct MixnodeSummaryBonded {
        pub(crate) count: i32,
        pub(crate) active: i32,
        pub(crate) inactive: i32,
        pub(crate) reserve: i32,
        pub(crate) last_updated_utc: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
    pub(crate) struct MixnodeSummaryBlacklisted {
        pub(crate) count: i32,
        pub(crate) last_updated_utc: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
    pub(crate) struct MixnodeSummaryHistorical {
        pub(crate) count: i32,
        pub(crate) last_updated_utc: String,
    }
}

pub(crate) mod gateway {
    use super::*;

    #[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
    pub(crate) struct GatewaySummary {
        pub(crate) bonded: GatewaySummaryBonded,
        pub(crate) blacklisted: GatewaySummaryBlacklisted,
        pub(crate) historical: GatewaySummaryHistorical,
        pub(crate) explorer: GatewaySummaryExplorer,
    }

    #[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
    pub(crate) struct GatewaySummaryExplorer {
        pub(crate) count: i32,
        pub(crate) last_updated_utc: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
    pub(crate) struct GatewaySummaryBonded {
        pub(crate) count: i32,
        pub(crate) last_updated_utc: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
    pub(crate) struct GatewaySummaryHistorical {
        pub(crate) count: i32,
        pub(crate) last_updated_utc: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
    pub(crate) struct GatewaySummaryBlacklisted {
        pub(crate) count: i32,
        pub(crate) last_updated_utc: String,
    }
}
