use cosmwasm_std::Addr;
use serde::{Deserialize, Serialize}; 
use crate::state::Service; 

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum QueryMsg {
    Greet {},
    QueryAll {},
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct GreetResp {
    pub message: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct InstantiateMsg {
    pub admins: Vec<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum ExecuteMsg {
    Announce { client_address: Addr, whitelist: Vec<String>, owner: Addr },
    Delete { }, 
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct ServicesInfo {
    pub owner: Addr, 
    pub services: Service,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct ServicesListResp {
    pub services: Vec<ServicesInfo>,
}
