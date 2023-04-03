use cosmwasm_std::Addr;
use nym_service_provider_directory_common::{
    msg::ServiceInfo, NymAddress, Service, ServiceId, ServiceType,
};

use super::helpers::nyms;

pub fn service_fixture() -> Service {
    Service {
        nym_address: NymAddress::new("nym"),
        service_type: ServiceType::NetworkRequester,
        owner: Addr::unchecked("steve"),
        block_height: 12345,
        deposit: nyms(100),
    }
}

pub fn service_fixture_with_address(nym_address: &str) -> Service {
    Service {
        nym_address: NymAddress::new(nym_address),
        service_type: ServiceType::NetworkRequester,
        owner: Addr::unchecked("steve"),
        block_height: 12345,
        deposit: nyms(100),
    }
}

pub fn service_info(service_id: ServiceId, nym_address: NymAddress, owner: Addr) -> ServiceInfo {
    ServiceInfo {
        service_id,
        service: Service {
            nym_address,
            service_type: ServiceType::NetworkRequester,
            owner,
            block_height: 12345,
            deposit: nyms(100),
        },
    }
}
