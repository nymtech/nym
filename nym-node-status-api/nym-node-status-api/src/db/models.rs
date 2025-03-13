use std::str::FromStr;

use crate::{
    http::{self, models::SummaryHistory},
    utils::{decimal_to_i64, NumericalCheckedCast},
};
use anyhow::Context;
use nym_contracts_common::Percent;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_network_defaults::DEFAULT_NYM_NODE_HTTP_PORT;
use nym_node_requests::api::v1::node::models::NodeDescription;
use nym_validator_client::{
    client::NymNodeDetails, models::NymNodeDescription, nym_api::SkimmedNode,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use strum_macros::{EnumString, FromRepr};
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;

macro_rules! serialize_opt_to_value {
    ($var:expr) => {{
        match $var {
            None => Ok(None),
            Some(ref value) => serde_json::to_value(value).map(Some).map_err(|err| {
                anyhow::anyhow!("Failed to serialize {}: {:?}", stringify!($var), err)
            }),
        }
    }};
}

pub(crate) struct GatewayInsertRecord {
    pub(crate) identity_key: String,
    pub(crate) bonded: bool,
    pub(crate) self_described: String,
    // TODO dz shouldn't be an option
    pub(crate) explorer_pretty_bond: Option<String>,
    pub(crate) last_updated_utc: i64,
    pub(crate) performance: u8,
}

#[derive(Debug, Clone)]
pub(crate) struct GatewayDto {
    pub(crate) gateway_identity_key: String,
    pub(crate) bonded: bool,
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
    pub(crate) full_details: String,
    pub(crate) self_described: Option<String>,
    pub(crate) last_updated_utc: i64,
    pub(crate) is_dp_delegatee: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct MixnodeDto {
    pub(crate) mix_id: i64,
    pub(crate) bonded: bool,
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
        let is_dp_delegatee = value.is_dp_delegatee;
        let moniker = value.moniker.clone();
        let website = value.website.clone();
        let security_contact = value.security_contact.clone();
        let details = value.details.clone();

        Ok(http::models::Mixnode {
            mix_id,
            bonded: value.bonded,
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

pub(crate) const MIXNODES_LEGACY_COUNT: &str = "mixnodes.legacy.count";
pub(crate) const NYMNODES_DESCRIBED_COUNT: &str = "nymnode.described.count";

pub(crate) const NYMNODE_COUNT: &str = "nymnode.total.count";
pub(crate) const ASSIGNED_ENTRY_COUNT: &str = "assigned.entry.count";
pub(crate) const ASSIGNED_EXIT_COUNT: &str = "assigned.exit.count";
pub(crate) const ASSIGNED_MIXING_COUNT: &str = "assigned.mixing.count";

pub(crate) const GATEWAYS_BONDED_COUNT: &str = "gateways.bonded.count";

pub(crate) const MIXNODES_HISTORICAL_COUNT: &str = "mixnodes.historical.count";
pub(crate) const GATEWAYS_HISTORICAL_COUNT: &str = "gateways.historical.count";

// `utoipa` goes crazy if you use module-qualified prefix as field type so we
//  have to import it
use gateway::GatewaySummary;
use mixnode::MixnodeSummary;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub(crate) struct NetworkSummary {
    pub(crate) total_nodes: i32,
    pub(crate) mixnodes: MixnodeSummary,
    pub(crate) gateways: GatewaySummary,
}

pub(crate) mod mixnode {
    use super::*;

    #[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
    pub(crate) struct MixnodeSummary {
        pub(crate) bonded: MixingNodesSummary,
        pub(crate) historical: MixnodeSummaryHistorical,
    }

    #[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
    pub(crate) struct MixingNodesSummary {
        pub(crate) count: i32,
        pub(crate) self_described: i32,
        pub(crate) legacy: i32,
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

        pub(crate) historical: GatewaySummaryHistorical,
    }

    #[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
    pub(crate) struct GatewaySummaryBonded {
        pub(crate) count: i32,
        pub(crate) entry: i32,
        pub(crate) exit: i32,
        pub(crate) last_updated_utc: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
    pub(crate) struct GatewaySummaryHistorical {
        pub(crate) count: i32,
        pub(crate) last_updated_utc: String,
    }
}

#[allow(dead_code)] // not dead code, this is SQL data model
#[derive(Debug, Clone)]
pub struct TestRunDto {
    pub id: i64,
    pub gateway_id: i64,
    pub status: i64,
    pub created_utc: i64,
    pub ip_address: String,
    pub log: String,
    pub last_assigned_utc: Option<i64>,
}

#[derive(Debug, Clone, strum_macros::Display, EnumString, FromRepr, PartialEq)]
#[repr(u8)]
pub(crate) enum TestRunStatus {
    Complete = 2,
    InProgress = 1,
    Queued = 0,
}

#[derive(Debug, Clone)]
pub struct GatewayIdentityDto {
    pub gateway_identity_key: String,
    pub bonded: bool,
}

#[allow(dead_code)] // it's not dead code but clippy doesn't detect usage in sqlx macros
#[derive(Debug, Clone)]
pub struct GatewayInfoDto {
    pub id: i64,
    pub gateway_identity_key: String,
    pub self_described: Option<String>,
    pub explorer_pretty_bond: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct GatewaySessionsRecord {
    pub gateway_identity_key: String,
    pub node_id: i64,
    pub day: Date,
    pub unique_active_clients: i64,
    pub session_started: i64,
    pub users_hashes: Option<String>,
    pub vpn_sessions: Option<String>,
    pub mixnet_sessions: Option<String>,
    pub unknown_sessions: Option<String>,
}

impl TryFrom<GatewaySessionsRecord> for http::models::SessionStats {
    type Error = anyhow::Error;

    fn try_from(value: GatewaySessionsRecord) -> Result<Self, Self::Error> {
        let users_hashes = value.users_hashes.clone().unwrap_or("null".to_string());
        let vpn_sessions = value.vpn_sessions.clone().unwrap_or("null".to_string());
        let mixnet_sessions = value.mixnet_sessions.clone().unwrap_or("null".to_string());
        let unknown_sessions = value.unknown_sessions.clone().unwrap_or("null".to_string());

        let users_hashes = serde_json::from_str(&users_hashes).unwrap_or(None);
        let vpn_sessions = serde_json::from_str(&vpn_sessions).unwrap_or(None);
        let mixnet_sessions = serde_json::from_str(&mixnet_sessions).unwrap_or(None);
        let unknown_sessions = serde_json::from_str(&unknown_sessions).unwrap_or(None);

        Ok(http::models::SessionStats {
            gateway_identity_key: value.gateway_identity_key.clone(),
            node_id: value.node_id as u32,
            day: value.day,
            unique_active_clients: value.unique_active_clients,
            session_started: value.session_started,
            users_hashes,
            vpn_sessions,
            mixnet_sessions,
            unknown_sessions,
        })
    }
}

#[derive(strum_macros::Display)]
pub(crate) enum ScrapeNodeKind {
    LegacyMixnode { mix_id: i64 },
    MixingNymNode { node_id: i64 },
    EntryExitNymNode { node_id: i64, identity_key: String },
}

impl ScrapeNodeKind {
    pub(crate) fn node_id(&self) -> &i64 {
        match self {
            ScrapeNodeKind::LegacyMixnode { mix_id } => mix_id,
            ScrapeNodeKind::MixingNymNode { node_id } => node_id,
            ScrapeNodeKind::EntryExitNymNode { node_id, .. } => node_id,
        }
    }
}

pub(crate) struct ScraperNodeInfo {
    pub node_kind: ScrapeNodeKind,
    pub hosts: Vec<String>,
    pub http_api_port: i64,
}

impl ScraperNodeInfo {
    pub(crate) fn contact_addresses(&self) -> Vec<String> {
        let mut urls = Vec::new();
        for host in &self.hosts {
            urls.append(&mut vec![
                format!("http://{}:{}", host, DEFAULT_NYM_NODE_HTTP_PORT),
                format!("http://{}:8000", host),
                format!("https://{}", host),
                format!("http://{}", host),
            ]);

            if self.http_api_port != DEFAULT_NYM_NODE_HTTP_PORT as i64 {
                urls.insert(0, format!("http://{}:{}", host, self.http_api_port));
            }
        }

        urls
    }

    pub(crate) fn node_id(&self) -> &i64 {
        self.node_kind.node_id()
    }
}

#[allow(dead_code)] // it's not dead code but clippy doesn't detect usage in sqlx macros
#[derive(sqlx::Decode, Debug)]
pub(crate) struct NymNodeDto {
    pub node_id: i64,
    pub ed25519_identity_pubkey: String,
    pub total_stake: i64,
    pub ip_addresses: serde_json::Value,
    pub mix_port: i64,
    pub x25519_sphinx_pubkey: String,
    pub node_role: serde_json::Value,
    pub supported_roles: serde_json::Value,
    pub entry: Option<serde_json::Value>,
    pub performance: String,
    pub self_described: Option<serde_json::Value>,
    pub bond_info: Option<serde_json::Value>,
}

#[allow(dead_code)] // it's not dead code but clippy doesn't detect usage in sqlx macros
#[derive(Debug)]
pub(crate) struct NymNodeInsertRecord {
    pub node_id: i64,
    pub ed25519_identity_pubkey: String,
    pub total_stake: i64,
    pub ip_addresses: serde_json::Value,
    pub mix_port: i64,
    pub x25519_sphinx_pubkey: String,
    pub node_role: serde_json::Value,
    pub supported_roles: serde_json::Value,
    pub performance: String,
    pub entry: Option<serde_json::Value>,
    pub self_described: Option<serde_json::Value>,
    pub bond_info: Option<serde_json::Value>,
    pub last_updated_utc: String,
}

impl NymNodeInsertRecord {
    pub fn new(
        skimmed_node: SkimmedNode,
        bond_info: Option<&NymNodeDetails>,
        self_described: Option<&NymNodeDescription>,
    ) -> anyhow::Result<Self> {
        let now = OffsetDateTime::now_utc().to_string();

        // if bond info is missing, set stake to 0
        let total_stake = bond_info
            .map(|info| decimal_to_i64(info.total_stake()))
            .unwrap_or(0);
        let entry = serialize_opt_to_value!(skimmed_node.entry)?;
        let bond_info = serialize_opt_to_value!(bond_info)?;
        let self_described = serialize_opt_to_value!(self_described)?;

        let record = Self {
            node_id: skimmed_node.node_id.into(),
            ed25519_identity_pubkey: skimmed_node.ed25519_identity_pubkey.to_base58_string(),
            total_stake,
            ip_addresses: serde_json::to_value(&skimmed_node.ip_addresses)?,
            mix_port: skimmed_node.mix_port as i64,
            x25519_sphinx_pubkey: skimmed_node.x25519_sphinx_pubkey.to_base58_string(),
            node_role: serde_json::to_value(&skimmed_node.role)?,
            supported_roles: serde_json::to_value(skimmed_node.supported_roles)?,
            performance: skimmed_node.performance.value().to_string(),
            entry,
            self_described,
            bond_info,
            last_updated_utc: now,
        };

        Ok(record)
    }
}

impl TryFrom<NymNodeDto> for SkimmedNode {
    type Error = anyhow::Error;

    fn try_from(other: NymNodeDto) -> Result<Self, Self::Error> {
        let node_id = u32::try_from(other.node_id).context("Invalid node_id in DB")?;
        let supported_roles =
            serde_json::from_value(other.supported_roles).context("supported_roles")?;
        let node_role = serde_json::from_value(other.node_role).context("node_role")?;
        let ip_addresses = serde_json::from_value(other.ip_addresses).context("ip_addresses")?;
        let entry = match other.entry {
            Some(raw) => Some(serde_json::from_value(raw).context("entry")?),
            None => None,
        };

        let skimmed_node = SkimmedNode {
            node_id,
            ed25519_identity_pubkey: ed25519::PublicKey::from_base58_string(
                other.ed25519_identity_pubkey,
            )
            .context("ed25519_identity_pubkey")?,
            ip_addresses,
            mix_port: other.mix_port.try_into()?,
            x25519_sphinx_pubkey: x25519::PublicKey::from_base58_string(other.x25519_sphinx_pubkey)
                .context("x25519_sphinx_pubkey")?,
            role: node_role,
            supported_roles,
            entry,
            performance: Percent::from_str(&other.performance).context("can't parse Percent")?,
        };

        Ok(skimmed_node)
    }
}

#[derive(Debug, Serialize, Deserialize, sqlx::Decode)]
pub struct NodeStats {
    pub packets_received: i64,
    pub packets_sent: i64,
    pub packets_dropped: i64,
}
