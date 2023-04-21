use cosmwasm_std::Addr;
use nym_name_service_common::{NameId, NameInfo, NymAddress, NymName, RegisteredName};

use super::helpers::nyms;

pub fn name_fixture_full(name: &str, nym_address: &str, owner: &str) -> RegisteredName {
    RegisteredName {
        name: NymName::new(name),
        nym_address: NymAddress::new(nym_address),
        owner: Addr::unchecked(owner),
        block_height: 12345,
        deposit: nyms(100),
    }
}

pub fn name_fixture() -> RegisteredName {
    name_fixture_full("my-service", "client_id.client_key@gateway_id", "steve")
}

pub fn name_fixture_name(name: &str) -> RegisteredName {
    name_fixture_full(name, "client_id.client_key@gateway_id", "steve")
}

pub fn name_info(name_id: NameId, name: NymName, nym_address: NymAddress, owner: Addr) -> NameInfo {
    NameInfo {
        name_id,
        name: name_fixture_full(name.as_str(), nym_address.as_str(), owner.as_str()),
    }
}
