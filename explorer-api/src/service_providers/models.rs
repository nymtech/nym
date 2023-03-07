use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct DirectoryServiceProvider {
    pub id: String,
    pub description: String,
    pub address: String,
    pub gateway: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct DirectoryService {
    pub id: String,
    pub description: String,
    pub items: Vec<DirectoryServiceProvider>,
}
