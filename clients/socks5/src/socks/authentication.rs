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

#[derive(Clone, Debug)]

pub(crate) struct Authenticator {
    allowed_users: Vec<User>,
    pub auth_methods: Vec<u8>, // DH TODO: this should not be public
}

impl Authenticator {
    pub(crate) fn new(auth_methods: Vec<u8>, allowed_users: Vec<User>) -> Authenticator {
        Authenticator {
            allowed_users,
            auth_methods,
        }
    }

    /// Check if username + password pair are valid
    pub fn is_allowed(&self, user: &User) -> bool {
        if self
            .auth_methods
            .contains(&(AuthenticationMethods::UserPass as u8))
        {
            self.allowed_users.contains(user)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_works() {
        let mut auth_methods: Vec<u8> = Vec::new();
        auth_methods.push(AuthenticationMethods::NoAuth as u8);
        auth_methods.push(AuthenticationMethods::UserPass as u8);

        let admin = User {
            username: "foo".to_string(),
            password: "bar".to_string(),
        };
        let allowed_users = vec![admin.clone()];

        let authenticator = Authenticator::new(auth_methods, allowed_users);

        assert!(authenticator.allowed_users.contains(&admin));
        assert!(authenticator
            .auth_methods
            .contains(&(AuthenticationMethods::NoAuth as u8)));
        assert!(authenticator
            .auth_methods
            .contains(&(AuthenticationMethods::UserPass as u8)));
    }

    mod without_user_and_password_auth_enabled {
        use super::*;

        #[test]
        fn user_pass_authentication_fails() {
            let auth_methods: Vec<u8> = Vec::new(); // it's empty

            let admin = User {
                username: "foo".to_string(),
                password: "bar".to_string(),
            };

            let allowed_users = vec![admin.clone()];

            let authenticator = Authenticator::new(auth_methods, allowed_users);

            assert!(!authenticator.is_allowed(&admin));
        }
    }

    #[cfg(test)]
    mod with_user_and_password_auth_enabled {
        use super::*;

        #[test]
        fn allowed_user_passes_authentication_check() {
            let mut auth_methods: Vec<u8> = Vec::new();
            auth_methods.push(AuthenticationMethods::UserPass as u8);

            let admin = User {
                username: "foo".to_string(),
                password: "bar".to_string(),
            };

            let allowed_users = vec![admin.clone()];

            let authenticator = Authenticator::new(auth_methods, allowed_users);

            assert!(authenticator.is_allowed(&admin));
        }

        #[test]
        fn disallowed_user_fails_authentication_check() {
            let mut auth_methods: Vec<u8> = Vec::new();
            auth_methods.push(AuthenticationMethods::UserPass as u8);

            let bad_user = User {
                username: "ashy".to_string(),
                password: "slashy".to_string(),
            };

            let allowed_users = Vec::new();

            let authenticator = Authenticator::new(auth_methods, allowed_users);

            assert!(!authenticator.is_allowed(&bad_user));
        }
    }
}
