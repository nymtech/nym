use super::*;

/// POST a new PresenceAnnouncement
pub fn post(req: &mut Request) -> IronResult<Response> {
    Ok(Response::with(status::Created))
}

/// A PresenceAnnouncement received from a node asks for entry into the system.
/// It's not really a "presence" insofar as other means (e.g. health-checks,
/// mixmining, staking etc) are used to determine actual presence, and whether
/// the node is doing the work it should be doing. A PresenceAnnouncement is
/// a node saying "hey, I exist, and am ready to participate".
struct PresenceAnnouncement {
    host: String,
    public_key: String,
    node_type: String,
}
