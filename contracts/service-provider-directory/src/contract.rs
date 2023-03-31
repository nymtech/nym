use crate::error::ContractError;
use crate::msg::{QueryMsg, InstantiateMsg, ExecuteMsg, ServicesListResp, ConfigResponse};
use crate::state::{SERVICES, Service, CONFIG, Config};
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Order
};
use cw2::set_contract_version;
use cosmwasm_std::Addr;

const CONTRACT_NAME: &str = "service-storage-poc";
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
        admin: msg.admin
    };  

    CONFIG.save(deps.storage, &config)?; 
    
    // TODO add proper responses 
    Ok(Response::new())
}

pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg
) -> Result<Response, ContractError> {
    use ExecuteMsg::*;

    match msg {
        Announce { client_address, whitelist, owner } => exec::announce(_deps, _info, client_address, whitelist, owner ),
        Delete { client_address } => exec::delete(_deps, _info, client_address), // TODO fix in line with the comment 
        UpdateScore { client_address, new_score } => exec::update_score(_deps, _info, client_address, new_score) // TODO once changed mapping from info.sender to client address ¬
    }
}

mod exec {
    use super::*; 

    pub fn announce(
        deps: DepsMut, 
        info: MessageInfo, 
        client_address: String, 
        whitelist: Vec<String>, 
        owner: Addr 
    ) -> Result<Response, ContractError> {
        
        let new_service = Service { 
            client_address: client_address.clone(), 
            whitelist, 
            uptime_score: 0, // init @ 0 - no score on new service 
            owner
        }; 

        SERVICES.save(deps.storage, client_address.clone(), &new_service)?; 

        Ok(Response::new()
            .add_attribute("action", "service announced")
        )   
    }

    pub fn update_score( 
        deps: DepsMut, 
        info: MessageInfo, 
        client_address: String, 
        new_score: i8
    ) -> Result<Response, ContractError> {

        let to_update = SERVICES.load(deps.storage, client_address.clone())?;

        // update score & save 

        Ok(Response::new()
            .add_attribute("action", "service updated")
        )

    }

    /* 
     * TODO change this so that it requires 
     *  
     *  - that it comes from the updator role 
     */ 
    pub fn delete( 
        deps: DepsMut, 
        info: MessageInfo, 
        client_address: String
    ) -> Result<Response, ContractError> {

        // ACL: check function call is coming from service.owner, if ! then fail 

        SERVICES.remove(deps.storage, client_address.clone()); 

        Ok(Response::new()
            .add_attribute("action", "service deleted")
        )
    }

}


pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;

    match msg {
        QueryAll {} => to_binary(&query::query_all(_deps, _env)?), 
        QueryConfig {} => to_binary(&query::query_config(_deps, _env)?)
    }
}

mod query {
    use crate::msg::ServicesInfo;

    use super::*;

    pub fn query_all(
        deps: Deps,
        _env: Env,
    ) -> StdResult<ServicesListResp> {
        let services = SERVICES
            .range(deps.storage, None, None, Order::Ascending)
            .map(|item| {
                item.map(|(owner, services)| ServicesInfo {
                    owner,
                    services
                })
            }) 
            .collect::<StdResult<Vec<_>>>()?;           
        let resp = ServicesListResp{ services }; 
        Ok(resp)
    }

    pub fn query_config(
        _deps: Deps, 
        _env: Env, 
    ) -> StdResult<ConfigResponse> {
        let config = CONFIG.load(_deps.storage)?; 
        let resp = ConfigResponse { 
            updater_role: config.updater_role, 
            admin: config.admin 
        };
        Ok(resp)
    }

} 

#[cfg(test)]
mod tests {
    use cosmwasm_std::Addr;
    use cw_multi_test::{App, ContractWrapper, Executor};
    use crate::msg::ServicesInfo;
    use super::*;

    #[test]
    fn set_config() {
        let mut app = App::default();
        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));

        let addr = app
            .instantiate_contract(
                code_id,
                Addr::unchecked("owner"),
                &InstantiateMsg { updater_role: Addr::unchecked("updater"), admin: Addr::unchecked("admin") }, 
                &[],
                "Contract",
                None,
            )
            .unwrap();
        
        let resp: ConfigResponse = app
            .wrap()
            .query_wasm_smart(addr, &QueryMsg::QueryConfig {})
            .unwrap();

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
        let mut app = App::default();
        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));
        let addr = app
            .instantiate_contract(
                code_id,
                Addr::unchecked("owner"),
                &InstantiateMsg{ updater_role: Addr::unchecked("updater"), admin: Addr::unchecked("admin") }, 
                &[],
                "Contract",
                None,
            )
            .unwrap();

        let resp = app
            .execute_contract(
                Addr::unchecked("owner"),
                addr.clone(), 
                &ExecuteMsg::Announce {
                    client_address: "nymAddress".to_owned(),
                    whitelist: vec!["domain.url".to_owned(), "domain2.url".to_owned()], 
                    owner: Addr::unchecked("owner") 
                }, 
                &[],
            )
            .unwrap();

        let wasm = resp.events.iter().find(|ev| ev.ty == "wasm").unwrap();
        assert_eq!(
            wasm.attributes
                    .iter()
                    .find(|attr| attr.key == "action")
                    .unwrap()
                    .value,
            "service announced"
        );

        let query: ServicesListResp = app.wrap()
        .query_wasm_smart(addr.clone(), &QueryMsg::QueryAll {  })
        .unwrap(); 

        let test_service: Service = Service {
            client_address: "nymAddress".to_string(), 
            whitelist: vec!["domain.url".to_owned(), "domain2.url".to_owned()], 
            owner: Addr::unchecked("owner"),
            uptime_score: 0
        };

        let expected = vec![
            ServicesInfo {
                owner: "nymAddress".to_string(), 
                services: test_service,
            }
        ];
   
        assert_eq!(
            query, 
            ServicesListResp {
                services: expected
            }
        );
    }

    #[test]
    fn delete_service() {
        let mut app = App::default();
        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));
        let addr = app
            .instantiate_contract(
                code_id,
                Addr::unchecked("owner"),
                &InstantiateMsg{ updater_role: Addr::unchecked("updater"), admin: Addr::unchecked("admin") }, 
                &[],
                "Contract",
                None,
            )
            .unwrap();

        app
            .execute_contract(
                Addr::unchecked("owner"),
                addr.clone(), 
                &ExecuteMsg::Announce {
                    client_address: "nymAddress".to_string(), 
                    whitelist: vec!["domain.url".to_owned(), "domain2.url".to_owned()], 
                    owner: Addr::unchecked("owner") 
                }, 
                &[],
            )
            .unwrap();
    
            let query: ServicesListResp = app.wrap()
                .query_wasm_smart(addr.clone(), &QueryMsg::QueryAll {  })
                .unwrap(); 
    
            let test_service: Service = Service {
                client_address: "nymAddress".to_string(), 
                whitelist: vec!["domain.url".to_owned(), "domain2.url".to_owned()], 
                owner: Addr::unchecked("owner"),
                uptime_score: 0
            };
    
            let expected = vec![
                ServicesInfo {
                    owner: "nymAddress".to_string(), 
                    services: test_service,
                }
            ];

            assert_eq!(
                query, 
                ServicesListResp {
                    services: expected
                }
            );

        let delete_resp = app
            .execute_contract(
                Addr::unchecked("owner"),
                addr.clone(), 
                &ExecuteMsg::Delete { 
                    client_address: "nymAddress".to_string() 
                }, 
                &[],
            )
            .unwrap();
            
            let wasm = delete_resp.events.iter().find(|ev| ev.ty == "wasm").unwrap();
            assert_eq!(
                wasm.attributes
                        .iter()
                        .find(|attr| attr.key == "action")
                        .unwrap()
                        .value,
                "service deleted"
            );
 
            let query: ServicesListResp = app.wrap()
            .query_wasm_smart(addr.clone(), &QueryMsg::QueryAll {  })
            .unwrap(); 
    
            let expected = vec![];
       
            assert_eq!(
                query, 
                ServicesListResp {
                    services: expected
                }
            );

    }

} 

