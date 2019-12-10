use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CocoPresence {
    host: String,
    pub_key: String,
    last_seen: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MixNodePresence{
    host: String,
    pub_key: String,
    layer: u64,
    last_seen: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MixProviderPresence{
    host: String,
    pub_key: String,
}

// Topology shows us the current state of the overall Nym network
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Topology {
    coco_nodes:        Vec<CocoPresence>,
    mix_nodes:         Vec<MixNodePresence>,
    mix_provider_nodes: Vec<MixProviderPresence>
}
