use cosmwasm_std::{Addr, Coin, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};
use serde::{Deserialize, Serialize};

use crate::error::Result;

pub mod config;
pub mod service_id_counter;
pub mod services;

// WIP
pub use config::*;
pub use service_id_counter::*;
pub use services::*;

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
        types::{Service, ServiceId},
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
