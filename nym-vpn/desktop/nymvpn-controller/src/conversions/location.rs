impl From<nymvpn_types::location::Location> for crate::proto::Location {
    fn from(value: nymvpn_types::location::Location) -> Self {
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

impl From<Vec<nymvpn_types::location::Location>> for crate::proto::Locations {
    fn from(value: Vec<nymvpn_types::location::Location>) -> Self {
        Self {
            location: value
                .into_iter()
                .map(crate::proto::Location::from)
                .collect(),
        }
    }
}

impl From<crate::proto::Locations> for Vec<nymvpn_types::location::Location> {
    fn from(value: crate::proto::Locations) -> Self {
        value
            .location
            .into_iter()
            .map(nymvpn_types::location::Location::from)
            .collect()
    }
}
