use crate::{
    error::ContractError,
    msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ServicesListResponse},
    state::{Config, Service, CONFIG, SERVICES},
};
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw2::set_contract_version;

// WIP(JON): can we get this through vergen instead?
const CONTRACT_NAME: &str = "crate:nym-service-provider-directory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        updater_role: msg.updater_role,
        admin: msg.admin,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Announce {
            client_address,
            service_type,
            owner,
        } => exec::announce(deps, env, info, client_address, service_type, owner),
        ExecuteMsg::Delete { service_id: sp_id } => exec::delete(deps, info, sp_id),
    }
}

mod exec {
    use super::*;
    use crate::state::{self, ClientAddress, ServiceId, ServiceType};

    pub fn announce(
        deps: DepsMut,
        env: Env,
        _info: MessageInfo,
        client_address: ClientAddress,
        service_type: ServiceType,
        owner: Addr,
    ) -> Result<Response, ContractError> {
        let new_service = Service {
            client_address: client_address.clone(),
            service_type,
            owner,
            block_height: env.block.height,
        };

        let sp_id = state::next_sp_id_counter(deps.storage)?;

        SERVICES.save(deps.storage, sp_id, &new_service)?;

        Ok(Response::new().add_attribute("action", "service announced"))
    }

    pub fn delete(
        deps: DepsMut,
        info: MessageInfo,
        sp_id: ServiceId,
    ) -> Result<Response, ContractError> {
        let service_to_delete = SERVICES.load(deps.storage, sp_id)?;

        if info.sender != service_to_delete.owner {
            return Err(ContractError::Unauthorized {
                sender: info.sender,
            });
        }

        SERVICES.remove(deps.storage, sp_id);

        Ok(Response::new().add_attribute("action", "service deleted"))
    }
}

pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryAll {} => to_binary(&query::query_all(_deps, _env)?),
        QueryMsg::QueryConfig {} => to_binary(&query::query_config(_deps, _env)?),
    }
}

mod query {
    use crate::msg::ServiceInfo;

    use super::*;

    pub fn query_all(deps: Deps, _env: Env) -> StdResult<ServicesListResponse> {
        let services = SERVICES
            .range(deps.storage, None, None, Order::Ascending)
            .map(|item| {
                item.map(|(sp_id, service)| ServiceInfo {
                    service_id: sp_id,
                    service,
                })
            })
            .collect::<StdResult<Vec<_>>>()?;
        Ok(ServicesListResponse { services })
    }

    pub fn query_config(deps: Deps, _env: Env) -> StdResult<ConfigResponse> {
        let config = CONFIG.load(deps.storage)?;
        Ok(config.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        msg::ServiceInfo,
        state::{ClientAddress, ServiceType},
        test_helpers::{get_attribute, TestSetup},
    };
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Addr,
    };

    // Test to instantiate the contract without using the test helpers and cw_multi_test.
    #[test]
    fn instantiate_contract_without_helpers() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("test0", &[]);

        let msg = InstantiateMsg {
            updater_role: Addr::unchecked("test1"),
            admin: Addr::unchecked("test2"),
        };

        instantiate(deps.as_mut(), env, info, msg).unwrap();
    }

    #[test]
    fn instantiate_contract_with_helpers() {
        TestSetup::new();
    }

    #[test]
    fn query_config() {
        let setup = TestSetup::new();
        let resp: ConfigResponse = setup.query(&QueryMsg::QueryConfig {});

        assert_eq!(
            resp,
            ConfigResponse {
                updater_role: Addr::unchecked("updater"),
                admin: Addr::unchecked("admin")
            }
        );
    }

    #[test]
    fn announce_and_query_service() {
        let owner = Addr::unchecked("owner");
        let client_address = ClientAddress::new("nymAddress");
        let mut setup = TestSetup::new();
        setup.announce_network_requester(client_address, owner);

        assert_eq!(
            setup.query_all(),
            ServicesListResponse {
                services: vec![ServiceInfo {
                    service_id: 1,
                    service: Service {
                        client_address,
                        service_type: ServiceType::NetworkRequester,
                        owner,
                        block_height: 12345,
                    },
                }]
            }
        );
    }

    #[test]
    fn delete_service() {
        let mut setup = TestSetup::new();
        let owner = Addr::unchecked("owner");
        setup.announce_network_requester(ClientAddress::new("nymAddress"), owner);
        assert!(!setup.query_all().services.is_empty());
        setup.delete(1, Addr::unchecked("owner")).unwrap();
        assert!(setup.query_all().services.is_empty());
    }

    #[test]
    fn only_owner_can_delete_service() {
        let mut setup = TestSetup::new();
        setup.announce_network_requester(ClientAddress::new("nymAddress"));
        assert!(!setup.query_all().services.is_empty());

        let delete_resp: ContractError = setup
            .delete(1, Addr::unchecked("not_owner"))
            .unwrap_err()
            .downcast()
            .unwrap();

        assert_eq!(
            delete_resp,
            ContractError::Unauthorized {
                sender: Addr::unchecked("not_owner")
            }
        );
    }

    #[test]
    fn delete_service_that_does_not_exist() {
        todo!();
    }

    #[test]
    fn service_id_increases_for_new_services() {
        todo!();
    }

    #[test]
    fn service_id_is_not_resused_when_deleting_and_then_adding_a_new_service() {
        todo!();
    }
}
