use serde::{Deserialize, Serialize};

#[allow(dead_code)] // it's not dead code but clippy doesn't detect usage in sqlx macros
#[derive(Debug, Clone)]
pub struct GatewayIdentityDto {
    pub gateway_identity_key: String,
    pub bonded: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct TestRun {
    pub id: u32,
    pub identity_key: String,
    pub status: String,
    pub log: String,
}
