use cosmwasm_std::Addr;

use crate::types::{NymAddress, Service, ServiceType};

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
