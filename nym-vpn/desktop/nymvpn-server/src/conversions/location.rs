impl From<crate::proto::Location> for nymvpn_types::location::Location {
    fn from(value: crate::proto::Location) -> Self {
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

impl From<crate::proto::ListLocationsResponse> for Vec<nymvpn_types::location::Location> {
    fn from(value: crate::proto::ListLocationsResponse) -> Self {
        value
            .locations
            .into_iter()
            .map(nymvpn_types::location::Location::from)
            .collect()
    }
}
