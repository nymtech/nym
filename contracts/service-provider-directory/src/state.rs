use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Service {
    pub client_address: String,
    pub standard_whitelist: bool,
    pub uptime_score: u8,
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Config {
    pub updater_role: Addr,
    pub admin: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const SERVICES: Map<String, Service> = Map::new("services");
