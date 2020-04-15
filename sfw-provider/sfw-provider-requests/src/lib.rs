pub mod auth_token;
pub mod requests;
pub mod responses;

pub const DUMMY_MESSAGE_CONTENT: &[u8] =
    b"[DUMMY MESSAGE] Wanting something does not give you the right to have it.";

// TODO: consideration for the future: should all request/responses have associated IDs
// for "async" API? However, TCP should ensure packets are received in order, so maybe
// it's not really required?
