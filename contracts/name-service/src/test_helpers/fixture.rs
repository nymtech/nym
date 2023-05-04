use cosmwasm_std::Addr;
use nym_name_service_common::{Address, NameEntry, NameId, NymName, RegisteredName};

use super::helpers::nyms;

pub fn name_fixture_full(name: &str, nym_address: &str, owner: &str) -> RegisteredName {
    RegisteredName {
        name: NymName::new(name).unwrap(),
        address: Address::new(nym_address),
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

pub fn name_entry(name_id: NameId, name: NymName, address: Address, owner: Addr) -> NameEntry {
    NameEntry {
        name_id,
        name: name_fixture_full(name.as_str(), address.as_str(), owner.as_str()),
    }
}
