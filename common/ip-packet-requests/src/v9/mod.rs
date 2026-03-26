pub const VERSION: u8 = 9;

// v9 uses the same wire format as v8. The version bump indicates
// the message was sent with LP framing (SphinxStream).
pub use super::v8::{request, response};
