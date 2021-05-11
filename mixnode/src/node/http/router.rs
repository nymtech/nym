#[get("/world")] // <- route attribute
pub(crate) fn world() -> &'static str {
    // <- request handler
    "hello, world!"
}
