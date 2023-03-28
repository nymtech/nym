use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{self, Config},
};
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

mod execute;
mod query;

// version info for migration info
const CONTRACT_NAME: &str = "crate:nym-service-provider-directory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config = Config {
        admin: msg.admin.clone(),
        deposit_required: msg.deposit_required.clone(),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    state::save_config(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("admin", msg.admin))
}

pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Announce {
            nym_address: client_address,
            service_type,
            owner,
        } => execute::announce(deps, env, info, client_address, service_type, owner),
        ExecuteMsg::Delete { service_id: sp_id } => execute::delete(deps, info, sp_id),
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ServiceId { service_id } => to_binary(&query::query_id(deps, service_id)?),
        QueryMsg::All {} => to_binary(&query::query_all(deps)?),
        QueryMsg::Config {} => to_binary(&query::query_config(deps)?),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        msg::ServiceInfo,
        test_helpers::{
            assert::{
                assert_config, assert_empty, assert_not_found, assert_service, assert_services,
            },
            fixture::service_fixture,
            helpers::{get_attribute, nyms},
        },
    };

    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Addr, Coin,
    };

    const DENOM: &str = "unym";

    #[test]
    fn instantiate_contract() {
        let mut deps = mock_dependencies();

        let admin = Addr::unchecked("bar");
        let msg = InstantiateMsg {
            admin: admin.clone(),
            deposit_required: Coin::new(100u128, DENOM),
        };
        let info = mock_info("creator", &[]);

        // Instantiate contract
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        // Check that it worked by querying the config, and checking that the list of services is
        // empty
        assert_config(deps.as_ref(), admin);
        assert_empty(deps.as_ref());
    }

    #[test]
    fn announce_fails_incorrect_deposit() {
        let mut deps = mock_dependencies();

        let admin = Addr::unchecked("admin");
        let msg = InstantiateMsg {
            admin: admin.clone(),
            deposit_required: nyms(100),
        };
        let info = mock_info("creator", &[]);

        // Instantiate contract
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        // Announce
        let msg = service_fixture().into_announce_msg();

        assert_eq!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info("user", &[nyms(99)]),
                msg.clone()
            )
            .unwrap_err(),
            ContractError::InsufficientDeposit {
                funds: 99u128.into(),
                deposit_required: nyms(100),
            }
        );

        assert_eq!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info("user", &[nyms(101)]),
                msg
            )
            .unwrap_err(),
            ContractError::TooLargeDeposit {
                funds: 101u128.into(),
                deposit_required: nyms(100),
            }
        );

        assert_config(deps.as_ref(), admin);
        assert_empty(deps.as_ref());
    }

    #[test]
    fn announce_success() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            admin: Addr::unchecked("admin"),
            deposit_required: nyms(100),
        };
        let info = mock_info("creator", &[]);

        // Instantiate contract
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        // Announce
        let msg = service_fixture().into_announce_msg();
        let info = mock_info("user", &[nyms(100)]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Check that the service has had service id assigned to it
        let expected_id = 1;
        let sp_id: u64 = get_attribute(res.clone(), "service_id").parse().unwrap();
        assert_eq!(sp_id, expected_id);
        assert_eq!(
            get_attribute(res, "service_type"),
            "network_requester".to_string()
        );

        // The expected announced service
        let expected_service = ServiceInfo {
            service_id: expected_id,
            service: service_fixture(),
        };
        assert_services(deps.as_ref(), &[expected_service.clone()]);
        assert_service(deps.as_ref(), &expected_service);
    }

    #[test]
    fn delete() {
        let mut deps = mock_dependencies();

        let admin = Addr::unchecked("admin");
        let msg = InstantiateMsg {
            admin: admin.clone(),
            deposit_required: Coin::new(100, "unym"),
        };
        let info = mock_info("creator", &[]);

        // Instantiate contract
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        // Announce
        // Note: Timmy announces on Steve's behalf (who is the owner of the service).
        let msg = service_fixture().into_announce_msg();
        let info = mock_info("timmy", &[nyms(100)]);
        assert!(info.sender != service_fixture().owner);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // The expected announced service
        let expected_id = 1;
        let expected_service = ServiceInfo {
            service_id: expected_id,
            service: service_fixture(),
        };
        assert_services(deps.as_ref(), &[expected_service.clone()]);

        // Removing someone else's service will fail
        let msg = ExecuteMsg::Delete {
            service_id: expected_id,
        };
        let info = mock_info("timmy", &[]);
        assert_eq!(
            execute(deps.as_mut(), mock_env(), info, msg).unwrap_err(),
            ContractError::Unauthorized {
                sender: Addr::unchecked("timmy")
            }
        );

        // Removing an non-existent service will fail
        let msg = ExecuteMsg::Delete {
            service_id: expected_id + 1,
        };
        let info_owner = MessageInfo {
            sender: service_fixture().owner,
            funds: vec![],
        };
        assert_eq!(
            execute(deps.as_mut(), mock_env(), info_owner.clone(), msg).unwrap_err(),
            ContractError::NotFound {
                service_id: expected_id + 1
            }
        );

        // Remove as correct owner succeeds
        let msg = ExecuteMsg::Delete {
            service_id: expected_id,
        };
        let res = execute(deps.as_mut(), mock_env(), info_owner, msg).unwrap();
        assert_eq!(get_attribute(res, "service_id"), expected_id.to_string());
        assert_services(deps.as_ref(), &[]);
        assert_not_found(deps.as_ref(), expected_id);
    }
}
