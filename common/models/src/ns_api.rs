use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestrunAssignment {
    /// has nothing to do with GW identity key. This is PK from `gateways` table
    pub testrun_id: i64,
    pub gateway_pk_id: i64,
}
