use cosmwasm_std::{Addr, Coin, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};
use serde::{Deserialize, Serialize};

use crate::error::Result;

pub mod config;
pub mod service_id_counter;
pub mod services;

// WIP
pub use config::*;
pub use services::*;
pub use service_id_counter::*;

/// The directory of services are indexed by [`ServiceId`].
pub type ServiceId = u32;

/// The type of services provider supported
#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Debug)]
pub enum ServiceType {
    NetworkRequester,
}

impl std::fmt::Display for ServiceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let service_type = match self {
            ServiceType::NetworkRequester => "network_requester",
        };
        write!(f, "{service_type}")
    }
}

/// The types of addresses supported.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum NymAddress {
    /// String representation of a nym address, which is of the form
    /// client_id.client_enc@gateway_id.
    Address(String),
    // For the future when we have a nym-dns contract
    //Name(String),
}

impl NymAddress {
    /// Create a new nym address.
    pub fn new(address: &str) -> Self {
        Self::Address(address.to_string())
    }

    pub fn as_str(&self) -> &str {
        match self {
            NymAddress::Address(address) => address,
        }
    }
}

impl ToString for NymAddress {
    fn to_string(&self) -> String {
        self.as_str().to_string()
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Service {
    /// The address of the service.
    pub nym_address: NymAddress,
    /// The service type.
    pub service_type: ServiceType,
    /// Service owner.
    pub owner: Addr,
    /// Block height at which the service was added.
    pub block_height: u64,
    /// The deposit used to announce the service.
    pub deposit: Coin,
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        coins,
        testing::{mock_dependencies, mock_env, mock_info},
        Order,
    };

    use crate::{
        msg::{ExecuteMsg, InstantiateMsg, ServiceInfo},
        test_helpers::{assert::assert_services, fixture::service_fixture, helpers::get_attribute},
    };

    use super::*;

    impl Service {
        pub fn into_announce_msg(self) -> ExecuteMsg {
            ExecuteMsg::Announce {
                nym_address: self.nym_address,
                service_type: self.service_type,
                owner: self.owner,
            }
        }
    }

    #[test]
    fn save_and_load_returns_a_key() {
        let mut deps = mock_dependencies();
        let admin = Addr::unchecked("bar");
        let msg = InstantiateMsg {
            admin: admin.clone(),
            deposit_required: Coin::new(100, "unym"),
        };
        let info = mock_info("creator", &[]);

        // Instantiate contract
        let res = crate::instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        // Announce
        let msg = service_fixture().into_announce_msg();
        let info = mock_info("anyone", &coins(100, "unym"));

        let res = crate::execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
        let sp_id: ServiceId = get_attribute(res.clone(), "service_id").parse().unwrap();
        assert_eq!(sp_id, 1);

        let s = services();
        let k = s.keys(&deps.storage, None, None, Order::Ascending);
        assert_eq!(k.count(), 1);
    }

    #[test]
    fn deleted_service_id_is_not_reused() {
        let mut deps = mock_dependencies();
        let admin = Addr::unchecked("bar");
        let msg = InstantiateMsg {
            admin: admin.clone(),
            deposit_required: Coin::new(100, "unym"),
        };
        let info = mock_info("creator", &[]);

        // Instantiate contract
        let res = crate::instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        // Announce
        let msg = service_fixture().into_announce_msg();
        let info = mock_info("anyone", &coins(100, "unym"));

        let res = crate::execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
        let sp_id: ServiceId = get_attribute(res.clone(), "service_id").parse().unwrap();
        assert_eq!(sp_id, 1);

        assert_services(deps.as_ref(), &[ServiceInfo::new(1, service_fixture())]);

        let res = crate::execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let sp_id: ServiceId = get_attribute(res.clone(), "service_id").parse().unwrap();
        assert_eq!(sp_id, 2);

        assert_services(
            deps.as_ref(),
            &[
                ServiceInfo::new(1, service_fixture()),
                ServiceInfo::new(2, service_fixture()),
            ],
        );

        // Delete the last entry
        let msg = ExecuteMsg::delete(2);
        let info = mock_info(&service_fixture().owner.to_string(), &[]);
        crate::execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_services(deps.as_ref(), &[ServiceInfo::new(1, service_fixture())]);

        // Create a third entry. The index should not reuse the previous entry that we just
        // deleted.
        let msg = service_fixture().into_announce_msg();
        let info = mock_info("anyone", &coins(100, "unym"));
        let res = crate::execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let sp_id: ServiceId = get_attribute(res.clone(), "service_id").parse().unwrap();
        assert_eq!(sp_id, 3);

        assert_services(
            deps.as_ref(),
            &[
                ServiceInfo::new(1, service_fixture()),
                ServiceInfo::new(3, service_fixture()),
            ],
        );
    }
}
