use rocket::Request;

pub(crate) mod verloc;

#[catch(404)]
pub(crate) fn not_found(req: &Request) -> String {
    format!("I couldn't find '{}'. Try something else?", req.uri())
}
