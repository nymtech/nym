use anyhow::Result;
use cosmwasm_std::{Addr, Coin};
use cw_multi_test::{App, AppResponse, ContractWrapper, Executor};
use serde::de::DeserializeOwned;

use crate::{
    msg::{
        ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ServiceInfo, ServicesListResponse,
    },
    state::{NymAddress, ServiceId, ServiceType},
    test_helpers::helpers::get_app_attribute,
};

// Helper for being able to systematic integration tests
pub struct TestSetup {
    app: App,
    addr: Addr,
}

impl TestSetup {
    pub fn new() -> Self {
        let mut app = App::default();
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
                updater_role: Addr::unchecked("updater"),
                admin: Addr::unchecked("admin"),
                deposit_required: Coin::new(100, "unym"),
            },
            &[],
            "contract_label",
            None,
        )
        .unwrap()
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
            &[],
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
}
