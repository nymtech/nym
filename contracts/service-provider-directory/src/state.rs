use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize}; 

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Service {
    pub client_address: Addr, 
    pub whitelist: Vec<String>, 
    pub uptime_score: u8,
    pub owner: Addr
}

pub const ADMINS: Item<Vec<Addr>> = Item::new("admins");
// pub const SERVICES: Item<Vec<Service>> = Item::new("services");
pub const SERVICES: Map<&Addr, Service> = Map::new("services"); 
