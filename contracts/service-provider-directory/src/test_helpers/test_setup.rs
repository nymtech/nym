use anyhow::Result;
use cosmwasm_std::{coins, Addr, Coin, StdResult, Uint128};
use cw_multi_test::{App, AppBuilder, AppResponse, ContractWrapper, Executor};
use serde::de::DeserializeOwned;

use crate::{
    msg::{
        ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ServiceInfo, ServicesListResponse,
    },
    state::{NymAddress, ServiceId, ServiceType},
    test_helpers::helpers::get_app_attribute,
};

const CONTRACT_DENOM: &str = "unym";

// Helper for being able to systematic integration tests
pub struct TestSetup {
    app: App,
    addr: Addr,
}

impl TestSetup {
    pub fn new() -> Self {
        //let mut app = App::default();
        let mut app = AppBuilder::new().build(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &Addr::unchecked("user"), coins(150, "unym"))
                .unwrap();
            router
                .bank
                .init_balance(storage, &Addr::unchecked("admin"), coins(150, "unym"))
                .unwrap();
            router
                .bank
                .init_balance(storage, &Addr::unchecked("owner"), coins(150, "unym"))
                .unwrap();
            router
                .bank
                .init_balance(storage, &Addr::unchecked("owner2"), coins(150, "unym"))
                .unwrap();
        });
        let code = ContractWrapper::new(crate::execute, crate::instantiate, crate::query);
        let code_id = app.store_code(Box::new(code));
        let addr = Self::instantiate(&mut app, code_id);
        TestSetup { app, addr }
    }

    fn instantiate(app: &mut App, code_id: u64) -> Addr {
        app.instantiate_contract(
            code_id,
            Addr::unchecked("owner"),
            &InstantiateMsg {
                admin: Addr::unchecked("admin"),
                deposit_required: Coin::new(100, "unym"),
            },
            &[],
            "contract_label",
            None,
        )
        .unwrap()
    }

    #[allow(unused)]
    pub fn address(&self) -> &Addr {
        &self.addr
    }

    pub fn contract_balance(&self) -> StdResult<Coin> {
        self.app.wrap().query_balance(&self.addr, "unym")
    }

    pub fn query<T: DeserializeOwned>(&self, query_msg: &QueryMsg) -> T {
        self.app
            .wrap()
            .query_wasm_smart(&self.addr, query_msg)
            .unwrap()
    }

    pub fn query_config(&self) -> ConfigResponse {
        self.query(&QueryMsg::QueryConfig {})
    }

    pub fn query_id(&self, service_id: ServiceId) -> ServiceInfo {
        self.query(&QueryMsg::QueryId { service_id })
    }

    pub fn query_all(&self) -> ServicesListResponse {
        self.query(&QueryMsg::QueryAll {})
    }

    pub fn announce_network_requester(
        &mut self,
        address: NymAddress,
        owner: Addr,
    ) -> Result<AppResponse> {
        let resp = self.app.execute_contract(
            owner.clone(),
            self.addr.clone(),
            &ExecuteMsg::Announce {
                nym_address: address,
                service_type: ServiceType::NetworkRequester,
                owner,
            },
            //&[],
            &[Coin {
                denom: "unym".to_string(),
                amount: Uint128::new(100),
            }],
        );
        if let Ok(ref resp) = resp {
            assert_eq!(get_app_attribute(&resp, "action"), "announce");
        }
        resp
    }

    pub fn delete(&mut self, service_id: ServiceId, owner: Addr) -> Result<AppResponse> {
        let delete_resp = self.app.execute_contract(
            owner,
            self.addr.clone(),
            &ExecuteMsg::Delete { service_id },
            &[],
        );
        if let Ok(ref resp) = delete_resp {
            assert_eq!(get_app_attribute(&resp, "action"), "delete");
        }
        delete_resp
    }

    pub fn balance(&self, address: impl Into<String>) -> StdResult<Coin> {
        self.app.wrap().query_balance(address, "unym")
    }
}
