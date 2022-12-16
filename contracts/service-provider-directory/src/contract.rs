use crate::error::ContractError;
use crate::msg::{GreetResp, QueryMsg, InstantiateMsg, ExecuteMsg, ServicesListResp};
use crate::state::{ADMINS, SERVICES, Service};
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Order
};
use cw2::set_contract_version;
use cosmwasm_std::Addr;

// version info for migration info
const CONTRACT_NAME: &str = "service-storage-poc";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let admins: StdResult<Vec<_>> = msg
        .admins
        .into_iter()
        .map(|addr| deps.api.addr_validate(&addr))
        .collect();
    ADMINS.save(deps.storage, &admins?)?;
    
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
        Delete { } => exec::delete(_deps, _info),
        /* TODO 
         * UpdateScore()
         * Edit { client_address, whitelist, owner } => exec::edit(_deps, _info, client_address, whitelist, owner),
        */
    }
}

mod exec {
    use super::*; 

    pub fn announce(
        deps: DepsMut, 
        info: MessageInfo, 
        client_address: Addr, 
        whitelist: Vec<String>, 
        owner: Addr 
    ) -> Result<Response, ContractError> {
        
        let new_service = Service { 
            client_address: client_address.clone(), 
            whitelist: whitelist, 
            uptime_score: 0, // init @ 0 - no score on new service 
            owner: owner
        }; 

        SERVICES.save(deps.storage, &info.sender, &new_service)?; 

        Ok(Response::new()
            .add_attribute("action", "service announced")
        )   
    }

    pub fn delete( 
        deps: DepsMut, 
        info: MessageInfo, 
    ) -> Result<Response, ContractError> {

        SERVICES.remove(deps.storage, &info.sender); 

        Ok(Response::new()
            .add_attribute("action", "service deleted")
        )
    }

}


pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;

    match msg {
        Greet {} => to_binary(&query::greet()?),
        QueryAll {} => to_binary(&query::query_all(_deps, _env)?)
    }
}

mod query {
    use crate::msg::ServicesInfo;

    use super::*;

    pub fn greet() -> StdResult<GreetResp> {
        let resp = GreetResp {
            message: "Hello World".to_owned(),
        };
        Ok(resp)
    }

    pub fn query_all(
        deps: Deps,
        _env: Env,
    ) -> StdResult<ServicesListResp> {
        let services = SERVICES
            .range(deps.storage, None, None, Order::Ascending)
            .map(|item| {
                item.map(|( owner, services) | ServicesInfo {
                    owner: owner.into(),
                    services: services
                })
            }) 
            .collect::<StdResult<Vec<_>>>()?;           
        let resp = ServicesListResp{ services }; 
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
    fn greet_query() {
        let mut app = App::default();
        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));

        let addr = app
            .instantiate_contract(
                code_id,
                Addr::unchecked("owner"),
                &InstantiateMsg { admins: vec![] }, 
                &[],
                "Contract",
                None,
            )
            .unwrap();

        let resp: GreetResp = app
            .wrap()
            .query_wasm_smart(addr, &QueryMsg::Greet {})
            .unwrap();

        assert_eq!(
            resp,
            GreetResp {
                message: "Hello World".to_owned()
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
                &InstantiateMsg { admins: vec![]}, 
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
                    client_address: Addr::unchecked("client address"), 
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
            client_address: Addr::unchecked("client address"),
            whitelist: vec!["domain.url".to_owned(), "domain2.url".to_owned()], 
            owner: Addr::unchecked("owner"),
            uptime_score: 0
        };

        let expected = vec![
            ServicesInfo {
                owner: Addr::unchecked("owner"), 
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
                &InstantiateMsg { admins: vec![]}, 
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
                    client_address: Addr::unchecked("client address"), 
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
                client_address: Addr::unchecked("client address"),
                whitelist: vec!["domain.url".to_owned(), "domain2.url".to_owned()], 
                owner: Addr::unchecked("owner"),
                uptime_score: 0
            };
    
            let expected = vec![
                ServicesInfo {
                    owner: Addr::unchecked("owner"), 
                    services: test_service,
                }
            ];

            assert_eq!(
                query, 
                ServicesListResp {
                    services: expected
                }
            );

        app
            .execute_contract(
                Addr::unchecked("owner"),
                addr.clone(), 
                &ExecuteMsg::Delete {  }, 
                &[],
            )
            .unwrap();

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

