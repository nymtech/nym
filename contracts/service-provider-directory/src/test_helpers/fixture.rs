use cosmwasm_std::Addr;
use nym_service_provider_directory_common::{NymAddress, Service, ServiceType};

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

pub fn service_fixture_by_name(nym_address: &str) -> Service {
    Service {
        nym_address: NymAddress::new(nym_address),
        service_type: ServiceType::NetworkRequester,
        owner: Addr::unchecked("steve"),
        block_height: 12345,
        deposit: nyms(100),
    }
}
