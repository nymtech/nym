use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};

// Storage keys
pub const CONFIG_KEY: &str = "config";
pub const SP_ID_COUNTER_KEY: &str = "spidc";
pub const SERVICES_KEY: &str = "services";

// Storage
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);
pub const SERVICES: Map<SpId, Service> = Map::new(SERVICES_KEY);
pub const SP_ID_COUNTER: Item<SpId> = Item::new(SP_ID_COUNTER_KEY);

pub type SpId = u64;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum ServiceType {
    NetworkRequester,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum ClientAddress {
    Address(String),
    // For the future when we have a nym-dns contract
    //Name(String),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Service {
    /// The address of the service.
    pub client_address: ClientAddress,
    /// The service type.
    pub service_type: ServiceType,
    /// Service owner.
    pub owner: Addr,
    /// Block height at which the service was added.
    pub block_height: u64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Config {
    pub updater_role: Addr,
    pub admin: Addr,
}

// Generate the next service provider id, store it and return it
pub(crate) fn next_sp_id_counter(store: &mut dyn Storage) -> StdResult<SpId> {
    // The first id is 1.
    let id: SpId = SP_ID_COUNTER.may_load(store)?.unwrap_or_default() + 1;
    SP_ID_COUNTER.save(store, &id)?;
    Ok(id)
}

