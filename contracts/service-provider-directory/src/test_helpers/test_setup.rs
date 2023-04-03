use anyhow::Result;
use cosmwasm_std::{coins, Addr, Coin, StdResult, Uint128};
use cw_multi_test::{App, AppBuilder, AppResponse, ContractWrapper, Executor};
use nym_service_provider_directory_common::{
    msg::{
        ConfigResponse, ExecuteMsg, InstantiateMsg, PagedServicesListResponse, QueryMsg,
        ServiceInfo,
    },
    NymAddress, ServiceId, ServiceType,
};
use serde::de::DeserializeOwned;

use crate::test_helpers::helpers::get_app_attribute;

const DENOM: &str = "unym";
const ADDRESSES: &[&str] = &[
    "user", "admin", "owner", "owner1", "owner2", "owner3", "owner4",
];

/// Helper for being able to systematic integration tests
pub struct TestSetup {
    app: App,
    addr: Addr,
}

impl Default for TestSetup {
    fn default() -> Self {
        TestSetup::new()
    }
}

impl TestSetup {
    pub fn new() -> Self {
        let mut app = AppBuilder::new().build(|router, _, storage| {
            let mut init_balance = |account: &str| {
                router
                    .bank
                    .init_balance(storage, &Addr::unchecked(account), coins(250, DENOM))
                    .unwrap();
            };
            ADDRESSES.iter().for_each(|addr| init_balance(addr));
        });
        let code = ContractWrapper::new(crate::execute, crate::instantiate, crate::query);
        let code_id = app.store_code(Box::new(code));
        let addr = Self::instantiate(&mut app, code_id);
        TestSetup { app, addr }
    }

    fn instantiate(app: &mut App, code_id: u64) -> Addr {
        app.instantiate_contract(
            code_id,
            Addr::unchecked("admin"),
            &InstantiateMsg {
                deposit_required: Coin::new(100, DENOM),
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
        self.app.wrap().query_balance(&self.addr, DENOM)
    }

    pub fn query<T: DeserializeOwned>(&self, query_msg: &QueryMsg) -> T {
        self.app
            .wrap()
            .query_wasm_smart(&self.addr, query_msg)
            .unwrap()
    }

    pub fn query_config(&self) -> ConfigResponse {
        self.query(&QueryMsg::Config {})
    }

    pub fn query_id(&self, service_id: ServiceId) -> ServiceInfo {
        self.query(&QueryMsg::ServiceId { service_id })
    }

    pub fn query_all(&self) -> PagedServicesListResponse {
        self.query(&QueryMsg::all())
    }

    pub fn announce_network_requester(
        &mut self,
        address: NymAddress,
        owner: Addr,
    ) -> Result<AppResponse> {
        let resp = self.app.execute_contract(
            owner,
            self.addr.clone(),
            &ExecuteMsg::Announce {
                nym_address: address,
                service_type: ServiceType::NetworkRequester,
            },
            &[Coin {
                denom: DENOM.to_string(),
                amount: Uint128::new(100),
            }],
        );
        if let Ok(ref resp) = resp {
            assert_eq!(
                get_app_attribute(&resp, "wasm-announce", "action"),
                "announce"
            );
        }
        resp
    }

    pub fn delete(&mut self, service_id: ServiceId, owner: Addr) -> Result<AppResponse> {
        let delete_resp = self.app.execute_contract(
            owner,
            self.addr.clone(),
            &ExecuteMsg::DeleteId { service_id },
            &[],
        );
        if let Ok(ref resp) = delete_resp {
            assert_eq!(
                get_app_attribute(&resp, "wasm-delete_id", "action"),
                "delete_id"
            );
        }
        delete_resp
    }

    pub fn delete_nym_address(
        &mut self,
        nym_address: NymAddress,
        owner: Addr,
    ) -> Result<AppResponse> {
        self.app.execute_contract(
            owner,
            self.addr.clone(),
            &ExecuteMsg::DeleteNymAddress { nym_address },
            &[],
        )
    }

    pub fn balance(&self, address: impl Into<String>) -> StdResult<Coin> {
        self.app.wrap().query_balance(address, DENOM)
    }
}
