use crate::state::{ClientAddress, Config, Service, ServiceType, SpId};
use cosmwasm_std::Addr;
use serde::{Deserialize, Serialize};

//#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
//pub struct GreetResp {
//    pub message: String,
//}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct InstantiateMsg {
    pub updater_role: Addr,
    pub admin: Addr,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum ExecuteMsg {
    Announce {
        client_address: ClientAddress,
        service_type: ServiceType,
        owner: Addr,
    },
    Delete {
        sp_id: SpId,
    },
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum QueryMsg {
    QueryAll {},
    QueryConfig {},
}


#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct ServiceInfo {
    pub sp_id: SpId,
    pub service: Service,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct ServicesListResponse {
    pub services: Vec<ServiceInfo>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct ConfigResponse {
    pub updater_role: Addr,
    pub admin: Addr,
}

impl From<Config> for ConfigResponse {
    fn from(config: Config) -> Self {
        ConfigResponse {
            updater_role: config.updater_role,
            admin: config.admin,
        }
    }
}
