use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
/// A notification sent to the validators to let them know whether a given mix is
/// currently up or down (based on whether it's mixing packets)
pub struct MixStatus {
    pub pub_key: String,
    pub ip_version: String,
    pub up: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
/// A notification sent to the validators to let them know whether a given set of mixes is
/// currently up or down (based on whether it's mixing packets)
pub struct BatchMixStatus {
    pub status: Vec<MixStatus>,
}
