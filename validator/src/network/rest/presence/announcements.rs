use super::*;
use chrono::NaiveDateTime;
use iron::status;

/// POST a new presence::Announcement
pub fn post(req: &mut Request) -> IronResult<Response> {
    Ok(Response::with(status::Created))
}

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
