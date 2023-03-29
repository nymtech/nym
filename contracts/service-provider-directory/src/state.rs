use cosmwasm_std::{Addr, Coin, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};
use serde::{Deserialize, Serialize};

use crate::error::Result;

// Storage keys
pub const CONFIG_KEY: &str = "config";
pub const SERVICE_ID_COUNTER_KEY: &str = "sidc";

// Storage
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);
pub const SERVICE_ID_COUNTER: Item<ServiceId> = Item::new(SERVICE_ID_COUNTER_KEY);

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

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Config {
    pub admin: Addr,
    pub deposit_required: Coin,
}

pub(crate) fn save_config(store: &mut dyn Storage, config: &Config) -> Result<()> {
    Ok(CONFIG.save(store, config)?)
}

pub(crate) fn load_config(store: &dyn Storage) -> Result<Config> {
    Ok(CONFIG.load(store)?)
}

/// Return the deposit required to announce a service.
pub(crate) fn deposit_required(store: &dyn Storage) -> Result<Coin> {
    Ok(CONFIG.load(store).map(|config| config.deposit_required)?)
}

/// Return the address of the contract admin
#[allow(unused)]
pub(crate) fn admin(store: &dyn Storage) -> Result<Addr> {
    Ok(CONFIG.load(store).map(|config| config.admin)?)
}

/// Generate the next service provider id, store it and return it
pub(crate) fn next_service_id_counter(store: &mut dyn Storage) -> Result<ServiceId> {
    // The first id is 1.
    let id = SERVICE_ID_COUNTER.may_load(store)?.unwrap_or_default() + 1;
    SERVICE_ID_COUNTER.save(store, &id)?;
    Ok(id)
}

pub(crate) struct ServiceIndex<'a> {
    pub(crate) nym_address: MultiIndex<'a, String, Service, ServiceId>,
    pub(crate) owner: MultiIndex<'a, Addr, Service, ServiceId>,
}

const SERVICES_PK_NAMESPACE: &str = "sernames";
const SERVICES_OWNER_IDX_NAMESPACE: &str = "serown";
const SERVICES_NYM_ADDRESS_IDX_NAMESPACE: &str = "sernyma";

impl<'a> IndexList<Service> for ServiceIndex<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Service>> + '_> {
        let v: Vec<&dyn Index<Service>> = vec![&self.nym_address, &self.owner];
        Box::new(v.into_iter())
    }
}

pub(crate) fn services<'a>() -> IndexedMap<'a, ServiceId, Service, ServiceIndex<'a>> {
    let indexes = ServiceIndex {
        nym_address: MultiIndex::new(
            |d| d.nym_address.to_string(),
            SERVICES_PK_NAMESPACE,
            SERVICES_NYM_ADDRESS_IDX_NAMESPACE,
        ),
        owner: MultiIndex::new(
            |d| d.owner.clone(),
            SERVICES_PK_NAMESPACE,
            SERVICES_OWNER_IDX_NAMESPACE,
        ),
    };
    IndexedMap::new(SERVICES_PK_NAMESPACE, indexes)
}

//pub(crate) fn all_services(store: &dyn Storage) -> Result<Vec<ServiceInfo>> {
//    SERVICES
//        .range(store, None, None, Order::Ascending)
//        .map(|item| {
//            item.map_err(ContractError::Std)
//                .map(|(service_id, service)| ServiceInfo {
//                    service_id,
//                    service,
//                })
//        })
//        .collect::<Result<Vec<_>>>()
//}

//pub(crate) fn remove_service(store: &mut dyn Storage, service_id: ServiceId) {
//    SERVICES.remove(store, service_id);
//}

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
