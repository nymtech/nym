use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::Item;
use mixnet_contract_common::ContractVersion;

use crate::constants::{CONTRACT_INFO_NAMESPACE, MAJOR, MINOR, PATCH};

pub const CONTRACT: Item<ContractVersion> = Item::new(CONTRACT_INFO_NAMESPACE);

pub fn set_contract_version(store: &mut dyn Storage) -> StdResult<()> {
    let val = ContractVersion {
        contract: "nym-mixnet-contract".to_string(),
        version: format!("{MAJOR}.{MINOR}.{PATCH}"),
    };
    CONTRACT.save(store, &val)
}

pub fn get_contract_version(store: &dyn Storage) -> StdResult<ContractVersion> {
    CONTRACT.load(store)
}
