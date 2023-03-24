use crate::{
    error::ContractError,
    msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ServicesListResponse},
    state::{Config, Service, CONFIG, SERVICES},
};
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw2::set_contract_version;

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
        updater_role: msg.updater_role.clone(),
        admin: msg.admin.clone(),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("admin", msg.admin)
        .add_attribute("updater_role", msg.updater_role))
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
        } => execute::announce(deps, env, client_address, service_type, owner),
        ExecuteMsg::Delete { service_id: sp_id } => execute::delete(deps, info, sp_id),
    }
}

pub mod execute {
    use super::*;
    use crate::state::{self, NymAddress, ServiceId, ServiceType};

    /// Announce a new service. It will be assigned a new service provider id.
    pub fn announce(
        deps: DepsMut,
        env: Env,
        nym_address: NymAddress,
        service_type: ServiceType,
        owner: Addr,
    ) -> Result<Response, ContractError> {
        let new_service = Service {
            nym_address,
            service_type: service_type.clone(),
            owner,
            block_height: env.block.height,
        };
        let service_id = state::next_service_id_counter(deps.storage)?;
        SERVICES.save(deps.storage, service_id, &new_service)?;
        Ok(Response::new()
            .add_attribute("action", "announce")
            .add_attribute("service_id", service_id.to_string())
            .add_attribute("service_type", service_type.to_string()))
    }

    /// Delete an exsisting service.
    pub fn delete(
        deps: DepsMut,
        info: MessageInfo,
        service_id: ServiceId,
    ) -> Result<Response, ContractError> {
        if !SERVICES.has(deps.storage, service_id) {
            return Err(ContractError::NotFound { service_id });
        }

        let service_to_delete = SERVICES.load(deps.storage, service_id)?;
        if info.sender != service_to_delete.owner {
            return Err(ContractError::Unauthorized {
                sender: info.sender,
            });
        }
        SERVICES.remove(deps.storage, service_id);
        Ok(Response::new()
            .add_attribute("action", "delete")
            .add_attribute("service_id", service_id.to_string()))
    }
}

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryId { service_id } => to_binary(&query::query_id(deps, env, service_id)?),
        QueryMsg::QueryAll {} => to_binary(&query::query_all(deps)?),
        QueryMsg::QueryConfig {} => to_binary(&query::query_config(deps)?),
    }
}

pub mod query {
    use super::*;
    use crate::{msg::ServiceInfo, state::ServiceId};

    pub fn query_id(deps: Deps, _env: Env, service_id: ServiceId) -> StdResult<ServiceInfo> {
        let service = SERVICES.load(deps.storage, service_id)?;
        Ok(ServiceInfo {
            service_id,
            service,
        })
    }

    pub fn query_all(deps: Deps) -> StdResult<ServicesListResponse> {
        let services = SERVICES
            .range(deps.storage, None, None, Order::Ascending)
            .map(|item| {
                item.map(|(service_id, service)| ServiceInfo {
                    service_id,
                    service,
                })
            })
            .collect::<StdResult<Vec<_>>>()?;
        Ok(ServicesListResponse { services })
    }

    pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
        let config = CONFIG.load(deps.storage)?;
        Ok(config.into())
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
            helpers::get_attribute,
        },
    };

    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Addr,
    };

    #[test]
    fn instantiate_contract() {
        let mut deps = mock_dependencies();

        let updater_role = Addr::unchecked("foo");
        let admin = Addr::unchecked("bar");
        let msg = InstantiateMsg {
            updater_role: updater_role.clone(),
            admin: admin.clone(),
        };
        let info = mock_info("creator", &[]);

        // Instantiate contract
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        // Check that it worked by querying the config, and checking that the list of services is
        // empty
        assert_config(deps.as_ref(), updater_role, admin);
        assert_empty(deps.as_ref());
    }

    #[test]
    fn announce() {
        let mut deps = mock_dependencies();

        let updater_role = Addr::unchecked("foo");
        let admin = Addr::unchecked("bar");
        let msg = InstantiateMsg {
            updater_role: updater_role.clone(),
            admin: admin.clone(),
        };
        let info = mock_info("creator", &[]);

        // Instantiate contract
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        // Announce
        let msg = service_fixture().into_announce_msg();
        let info = mock_info("anyone", &[]);
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

        let updater_role = Addr::unchecked("foo");
        let admin = Addr::unchecked("bar");
        let msg = InstantiateMsg {
            updater_role: updater_role.clone(),
            admin: admin.clone(),
        };
        let info = mock_info("creator", &[]);

        // Instantiate contract
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        // Announce (note: timmy announces on someone else's behalf)
        let msg = service_fixture().into_announce_msg();
        let info = mock_info("timmy", &[]);
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
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(
            res,
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
        let res = execute(deps.as_mut(), mock_env(), info_owner.clone(), msg).unwrap_err();
        assert_eq!(
            res,
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
