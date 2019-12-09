use serde::Deserialize;

#[derive(Deserialize)]
pub struct 	CocoHostInfo{
    host_info: HostInfo,
}

#[derive(Deserialize)]
pub struct CocoPresence {
    coco_host_info: CocoHostInfo,
    last_seen: i64,
}

#[derive(Deserialize)]
pub struct HostInfo {
    host: String,
    pub_key: String,
}

#[derive(Deserialize)]
pub struct MixHostInfo {
    host_info: HostInfo,
    layer: u64,
}

#[derive(Deserialize)]
pub struct MixNodePresence{
    mix_host_info: MixHostInfo,
    last_seen: i64,
}

#[derive(Deserialize)]
pub struct MixProviderPresence{
    mix_provider_host_info: MixProviderHostInfo,

}

#[derive(Deserialize)]
pub struct MixProviderHostInfo{
    host_info: HostInfo,
    last_seen: i64,
}

// Topology shows us the current state of the overall Nym network
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Topology {
    coco_nodes:        Vec<CocoPresence>,
    mix_nodes:         Vec<MixNodePresence>,
    mix_provider_nodes: Vec<MixProviderPresence>
}
