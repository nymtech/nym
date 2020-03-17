use super::*;

// POST a new PresenceAnnouncement
pub fn post(req: &mut Request) -> IronResult<Response> {
    Ok(Response::with(status::Created))
}

struct PresenceAnnouncement {
    host: String,
    public_key: String,
    node_type: String,
}
