use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestrunAssignment {
    pub testrun_id: i64,
    pub gateway_identity_key: String,
}
