use cosmwasm_std::Addr;
use serde::{Deserialize, Serialize}; 
use crate::state::Service; 

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum QueryMsg {
    QueryAll {},
    QueryConfig {},
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct GreetResp {
    pub message: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct InstantiateMsg {
    pub updater_role: Addr, 
    pub admin: Addr
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum ExecuteMsg {
    Announce { client_address: String, whitelist: Vec<String>, owner: Addr },
    Delete { client_address: String }, 
    UpdateScore { client_address: String, new_score: i8 }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct ServicesInfo {
    pub owner: String, 
    pub services: Service,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct ServicesListResp {
    pub services: Vec<ServicesInfo>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct ConfigResponse {
    pub updater_role: Addr, 
    pub admin: Addr
} 


