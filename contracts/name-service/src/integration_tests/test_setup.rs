use cosmwasm_std::{coins, Addr, Coin, Uint128};
use cw_multi_test::{App, AppBuilder, AppResponse, ContractWrapper, Executor};
use nym_contracts_common::signing::Nonce;
use nym_name_service_common::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    response::{ConfigResponse, PagedNamesListResponse},
    signing_types::{construct_name_register_sign_payload, SignableNameRegisterMsg},
    Address, NameDetails, NameId, NymName, RegisteredName,
};
use rand_chacha::ChaCha20Rng;
use serde::de::DeserializeOwned;

use crate::test_helpers::helpers::{get_app_attribute, test_rng};

use super::test_name::{SignedTestName, TestName};

const DENOM: &str = "unym";
const ADDRESSES: &[&str] = &[
    "user", "admin", "owner", "owner1", "owner2", "owner3", "owner4",
];
const WEALTHY_ADDRESSES: &[&str] = &["wealthy_owner_1", "wealthy_owner_2"];

/// Helper for being able to systematic integration tests
pub struct TestSetup {
    app: App,
    addr: Addr,
    rng: ChaCha20Rng,
}

impl Default for TestSetup {
    fn default() -> Self {
        TestSetup::new()
    }
}

impl TestSetup {
    pub fn new() -> Self {
        let mut app = AppBuilder::new().build(|router, _, storage| {
            let mut init_balance = |account: &str, amount: u128| {
                router
                    .bank
                    .init_balance(storage, &Addr::unchecked(account), coins(amount, DENOM))
                    .unwrap();
            };
            ADDRESSES.iter().for_each(|addr| init_balance(addr, 250));
            WEALTHY_ADDRESSES
                .iter()
                .for_each(|addr| init_balance(addr, 1000));
        });
        let code = ContractWrapper::new(crate::execute, crate::instantiate, crate::query);
        let code_id = app.store_code(Box::new(code));
        let addr = Self::instantiate(&mut app, code_id);
        let rng = test_rng();
        TestSetup { app, addr, rng }
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

    pub fn contract_balance(&self) -> Coin {
        self.app.wrap().query_balance(&self.addr, DENOM).unwrap()
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

    pub fn query_id(&self, name_id: NameId) -> RegisteredName {
        self.query(&QueryMsg::NameId { name_id })
    }

    pub fn query_all(&self) -> PagedNamesListResponse {
        self.query(&QueryMsg::all())
    }

    pub fn query_all_with_limit(
        &self,
        limit: Option<u32>,
        start_after: Option<u32>,
    ) -> PagedNamesListResponse {
        self.query(&QueryMsg::All { limit, start_after })
    }

    pub fn query_signing_nonce(&self, address: String) -> Nonce {
        self.query(&QueryMsg::SigningNonce { address })
    }

    pub fn new_name(&mut self, name: &NymName, address: &Address) -> TestName {
        TestName::new(&mut self.rng, name.clone(), address.clone())
    }

    pub fn payload_to_sign(
        &mut self,
        owner: &Addr,
        deposit: &Coin,
        name: &NameDetails,
    ) -> SignableNameRegisterMsg {
        let nonce = self.query_signing_nonce(owner.to_string());
        construct_name_register_sign_payload(nonce, owner.clone(), deposit.clone(), name.clone())
    }

    pub fn new_signed_name(
        &mut self,
        name: &NymName,
        address: &Address,
        owner: &Addr,
        deposit: &Coin,
    ) -> SignedTestName {
        let name = self.new_name(name, address);
        let payload = self.payload_to_sign(owner, deposit, name.details());
        name.sign(payload)
    }

    pub fn try_register(
        &mut self,
        name: &SignedTestName,
        owner: &Addr,
    ) -> anyhow::Result<AppResponse> {
        self.app.execute_contract(
            owner.clone(),
            self.addr.clone(),
            &ExecuteMsg::Register {
                name: name.name.clone(),
                owner_signature: name.owner_signature.clone(),
            },
            &[Coin {
                denom: DENOM.to_string(),
                amount: Uint128::new(100),
            }],
        )
    }

    pub fn register(&mut self, name: &SignedTestName, owner: &Addr) -> AppResponse {
        let resp = self.try_register(name, owner).unwrap();
        assert_eq!(
            get_app_attribute(&resp, "wasm-register", "action"),
            "register"
        );
        resp
    }

    // Convenience function for creating a new signed name, and regsitering it
    pub fn sign_and_register(
        &mut self,
        name: &NymName,
        address: &Address,
        owner: &Addr,
        deposit: &Coin,
    ) -> SignedTestName {
        let signed_name = self.new_signed_name(name, address, owner, deposit);
        self.register(&signed_name, owner);
        signed_name
    }

    pub fn try_delete(&mut self, name_id: NameId, owner: Addr) -> anyhow::Result<AppResponse> {
        self.app.execute_contract(
            owner,
            self.addr.clone(),
            &ExecuteMsg::DeleteId { name_id },
            &[],
        )
    }

    pub fn delete(&mut self, name_id: NameId, owner: Addr) -> AppResponse {
        let delete_resp = self.try_delete(name_id, owner).unwrap();
        assert_eq!(
            get_app_attribute(&delete_resp, "wasm-delete_id", "action"),
            "delete_id"
        );
        delete_resp
    }

    pub fn delete_name(&mut self, name: NymName, owner: Addr) -> AppResponse {
        self.app
            .execute_contract(
                owner,
                self.addr.clone(),
                &ExecuteMsg::DeleteName { name },
                &[],
            )
            .unwrap()
    }

    pub fn balance(&self, address: impl Into<String>) -> Coin {
        self.app.wrap().query_balance(address, DENOM).unwrap()
    }
}
