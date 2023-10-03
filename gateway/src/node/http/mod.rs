use rocket::{catch, Request};

mod client_registry;

pub use client_registry::*;

#[catch(404)]
pub(crate) fn not_found(req: &Request<'_>) -> String {
    format!("I couldn't find '{}'. Try something else?", req.uri())
}
