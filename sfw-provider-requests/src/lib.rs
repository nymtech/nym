pub mod requests;
pub mod responses;

pub const DUMMY_MESSAGE_CONTENT: &[u8] =
    b"[DUMMY MESSAGE] Wanting something does not give you the right to have it.";

pub type AuthToken = [u8; 32];
