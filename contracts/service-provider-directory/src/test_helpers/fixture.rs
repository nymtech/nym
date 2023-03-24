use cosmwasm_std::Addr;

use crate::state::{NymAddress, Service, ServiceType};

pub fn service_fixture() -> Service {
    Service {
        nym_address: NymAddress::new("nym"),
        service_type: ServiceType::NetworkRequester,
        owner: Addr::unchecked("steve"),
        block_height: 12345,
    }
}
