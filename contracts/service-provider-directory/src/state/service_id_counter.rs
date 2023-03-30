use cosmwasm_std::Storage;
use cw_storage_plus::Item;

use crate::{error::Result, types::ServiceId};

const SERVICE_ID_COUNTER_KEY: &str = "sidc";
const SERVICE_ID_COUNTER: Item<ServiceId> = Item::new(SERVICE_ID_COUNTER_KEY);

/// Generate the next service provider id, store it and return it
pub(crate) fn next_service_id_counter(store: &mut dyn Storage) -> Result<ServiceId> {
    // The first id is 1.
    let id = SERVICE_ID_COUNTER.may_load(store)?.unwrap_or_default() + 1;
    SERVICE_ID_COUNTER.save(store, &id)?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        coins,
        testing::{mock_dependencies, mock_env, mock_info},
        Coin,
    };

    use crate::{
        msg::{ExecuteMsg, InstantiateMsg, ServiceInfo},
        test_helpers::{assert::assert_services, fixture::service_fixture, helpers::get_attribute},
        types::ServiceId,
    };

    #[test]
    fn deleted_service_id_is_not_reused() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            deposit_required: Coin::new(100, "unym"),
        };
        let info = mock_info("creator", &[]);

        // Instantiate contract
        let res = crate::instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        // Announce
        let msg = service_fixture().into_announce_msg();
        let info = mock_info(service_fixture().owner.as_str(), &coins(100, "unym"));

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
        let info = mock_info(service_fixture().owner.as_str(), &coins(100, "unym"));
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
