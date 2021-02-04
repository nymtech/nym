use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MixNode {
    #[serde(flatten)]
    pub(crate) layer: u64,
    pub(crate) location: String,
    pub(crate) host: String,
    pub(crate) sphinx_key: String,
    pub(crate) version: String,
}
