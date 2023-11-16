impl From<crate::vpn_session::Model> for nymvpn_types::location::Location {
    fn from(value: crate::vpn_session::Model) -> Self {
        Self {
            code: value.location_code,
            country: value.location_country,
            country_code: value.location_country_code,
            city: value.location_city,
            city_code: value.location_city_code,
            state: value.location_state,
            state_code: value.location_state_code,
        }
    }
}

impl From<crate::recent_locations::Model> for nymvpn_types::location::Location {
    fn from(value: crate::recent_locations::Model) -> Self {
        Self {
            code: value.code,
            country: value.country,
            country_code: value.country_code,
            city: value.city,
            city_code: value.city_code,
            state: value.state,
            state_code: value.state_code,
        }
    }
}
