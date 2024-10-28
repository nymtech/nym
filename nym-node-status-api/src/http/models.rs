use crate::db::models::TestRunDto;
use nym_node_requests::api::v1::node::models::NodeDescription;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub(crate) use nym_bin_common::models::ns_api::TestrunAssignment;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Gateway {
    pub gateway_identity_key: String,
    pub bonded: bool,
    pub blacklisted: bool,
    pub performance: u8,
    pub self_described: Option<serde_json::Value>,
    pub explorer_pretty_bond: Option<serde_json::Value>,
    pub description: NodeDescription,
    pub last_probe_result: Option<serde_json::Value>,
    pub last_probe_log: Option<String>,
    pub last_testrun_utc: Option<String>,
    pub last_updated_utc: String,
    pub routing_score: f32,
    pub config_score: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct GatewaySkinny {
    pub gateway_identity_key: String,
    pub self_described: Option<serde_json::Value>,
    pub explorer_pretty_bond: Option<serde_json::Value>,
    pub last_probe_result: Option<serde_json::Value>,
    pub last_testrun_utc: Option<String>,
    pub last_updated_utc: String,
    pub routing_score: f32,
    pub config_score: u32,
    pub performance: u8,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct Mixnode {
    pub mix_id: u32,
    pub bonded: bool,
    pub blacklisted: bool,
    pub is_dp_delegatee: bool,
    pub total_stake: i64,
    pub full_details: Option<serde_json::Value>,
    pub self_described: Option<serde_json::Value>,
    pub description: NodeDescription,
    pub last_updated_utc: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct DailyStats {
    pub date_utc: String,
    pub total_packets_received: i64,
    pub total_packets_sent: i64,
    pub total_packets_dropped: i64,
    pub total_stake: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct Service {
    pub gateway_identity_key: String,
    pub last_updated_utc: String,
    pub routing_score: f32,
    pub service_provider_client_id: Option<String>,
    pub ip_address: Option<String>,
    pub hostname: Option<String>,
    pub mixnet_websockets: Option<serde_json::Value>,
    pub last_successful_ping_utc: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub(crate) struct SummaryHistory {
    pub date: String,
    pub value_json: serde_json::Value,
    pub timestamp_utc: String,
}

impl From<TestRunDto> for TestrunAssignment {
    fn from(value: TestRunDto) -> Self {
        Self {
            gateway_pk_id: value.gateway_id,
            testrun_id: value.id,
        }
    }
}
