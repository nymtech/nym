impl From<crate::proto::SignInRequest> for nymvpn_types::nymvpn_server::UserCredentials {
    fn from(value: crate::proto::SignInRequest) -> Self {
        Self {
            email: value.email,
            password: value.password,
        }
    }
}
