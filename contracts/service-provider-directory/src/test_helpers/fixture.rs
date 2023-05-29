use cosmwasm_std::Addr;
use nym_service_provider_directory_common::{
    NymAddress, Service, ServiceDetails, ServiceId, ServiceType,
};

use super::helpers::nyms;

pub fn service_fixture() -> ServiceDetails {
    ServiceDetails {
        nym_address: NymAddress::new("nym"),
        service_type: ServiceType::NetworkRequester,
        identity_key: "identity".to_string(),
    }
}

pub fn service_fixture_with_address(service_id: ServiceId, nym_address: &str) -> ServiceDetails {
    ServiceDetails {
        nym_address: NymAddress::new(nym_address),
        service_type: ServiceType::NetworkRequester,
        identity_key: "identity".to_string(),
    }
}

pub fn service_info(service_id: ServiceId, nym_address: NymAddress, announcer: Addr) -> Service {
    Service {
        service_id,
        service: ServiceDetails {
            nym_address,
            service_type: ServiceType::NetworkRequester,
            identity_key: "identity".to_string(),
        },
        announcer,
        block_height: 12345,
        deposit: nyms(100),
    }
}
