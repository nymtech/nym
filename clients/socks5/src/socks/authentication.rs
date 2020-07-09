use serde::Deserialize;

/// Client Authentication Methods
pub(crate) enum AuthenticationMethods {
    /// No Authentication
    NoAuth = 0x00,
    // GssApi = 0x01,
    /// Authenticate with a username / password
    UserPass = 0x02,
    /// Cannot authenticate
    NoMethods = 0xFF,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct User {
    pub username: String,
    pub password: String,
}

pub(crate) struct Authenticator {
    allowed_users: Vec<User>,
    auth_methods: Vec<u8>,
}
