use anyhow::Result;
use cosmwasm_std::{Addr, Response};
use cw_multi_test::{App, AppResponse, ContractWrapper, Executor};
use serde::de::DeserializeOwned;

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ServicesListResponse},
    state::{NymAddress, ServiceId, ServiceType},
};

pub mod assert;
pub mod fixture;

pub fn get_attribute(res: Response, key: &str) -> String {
    res.attributes
        .iter()
        .find(|attr| attr.key == key)
        .unwrap()
        .value
        .clone()
}

pub fn get_app_attribute(response: &AppResponse, key: &str) -> String {
    let wasm = response.events.iter().find(|ev| ev.ty == "wasm").unwrap();
    wasm.attributes
        .iter()
        .find(|attr| attr.key == key)
        .unwrap()
        .value
        .clone()
}

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
