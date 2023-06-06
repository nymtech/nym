use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct DirectoryServiceProvider {
    pub id: String,
    pub description: String,
    pub address: String,
    pub gateway: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct DirectorySpDetailed {
    pub id: String,
    pub description: String,
    pub address: String,
    pub gateway: String,
    pub routing_score: Option<f32>,
    pub service_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct DirectoryService {
    pub id: String,
    pub description: String,
    pub items: Vec<DirectoryServiceProvider>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HarbourMasterService {
    pub service_provider_client_id: String,
    pub gateway_identity_key: String,
    pub ip_address: String,
    pub last_successful_ping_utc: String,
    pub last_updated_utc: String,
    pub routing_score: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PagedResult<T> {
    pub page: u32,
    pub size: u32,
    pub total: i32,
    pub items: Vec<T>,
}
