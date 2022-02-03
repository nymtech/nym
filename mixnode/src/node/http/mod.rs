pub(crate) mod description;
pub(crate) mod stats;
pub(crate) mod verloc;

use rocket::Request;

#[catch(404)]
pub(crate) fn not_found(req: &Request<'_>) -> String {
    format!("I couldn't find '{}'. Try something else?", req.uri())
}
