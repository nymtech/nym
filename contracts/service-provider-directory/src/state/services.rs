use cosmwasm_std::Addr;
use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex};
use nym_service_provider_directory_common::{Service, ServiceId};

const SERVICES_PK_NAMESPACE: &str = "sernames";
const SERVICES_OWNER_IDX_NAMESPACE: &str = "serown";
const SERVICES_NYM_ADDRESS_IDX_NAMESPACE: &str = "sernyma";

pub(crate) struct ServiceIndex<'a> {
    pub(crate) nym_address: MultiIndex<'a, String, Service, ServiceId>,
    pub(crate) owner: MultiIndex<'a, Addr, Service, ServiceId>,
}

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

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        coins,
        testing::{mock_dependencies, mock_env, mock_info},
        Coin, Order,
    };
    use nym_service_provider_directory_common::ServiceId;

    use crate::{
        msg::{ExecuteMsg, InstantiateMsg},
        test_helpers::{fixture::service_fixture, helpers::get_attribute},
    };

    #[test]
    fn save_and_load_returns_a_key() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            deposit_required: Coin::new(100, "unym"),
        };
        let info = mock_info("creator", &[]);

        // Instantiate contract
        let res = crate::instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        // Announce
        let msg: ExecuteMsg = service_fixture().into();
        let info = mock_info("anyone", &coins(100, "unym"));

        let res = crate::execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
        let sp_id: ServiceId = get_attribute(res.clone(), "service_id").parse().unwrap();
        assert_eq!(sp_id, 1);

        let s = super::services();
        let k = s.keys(&deps.storage, None, None, Order::Ascending);
        assert_eq!(k.count(), 1);
    }
}
