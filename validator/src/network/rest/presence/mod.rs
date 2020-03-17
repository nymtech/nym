use super::*;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

pub mod announcements;
pub mod topology;

/// A presence::Announcement received from a node asks for entry into the system.
/// It's not really a "presence" insofar as other means (e.g. health-checks,
/// mixmining, staking etc) are used to determine actual presence, and whether
/// the node is doing the work it should be doing. A presence::Announcement is
/// a node saying "hey, I exist, and am ready to participate, but you need to
/// figure out if I should be made active by the system."
struct Announcement {
    host: String,
    public_key: String,
    node_type: String,
    seen_at: NaiveDateTime,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MixNode {
    host: String,
    public_key: String,
    version: String,
    last_seen: u64,
    location: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServiceProvider {
    host: String,
    public_key: String,
    version: String,
    last_seen: u64,
    location: String,
}

/// Topology shows us the current state of the overall Nym network
#[derive(Serialize, Deserialize, Debug)]
pub struct Topology {
    pub mix_nodes: Vec<MixNode>,
    pub service_providers: Vec<ServiceProvider>,
    pub validators: Vec<Validator>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Validator {
    host: String,
    public_key: String,
    version: String,
    last_seen: u64,
    location: String,
}
