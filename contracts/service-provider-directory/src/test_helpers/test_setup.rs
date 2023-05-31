use anyhow::Result;
use cosmwasm_std::{coins, Addr, Coin, Uint128};
use cw_multi_test::{App, AppBuilder, AppResponse, ContractWrapper, Executor};
use nym_contracts_common::signing::Nonce;
use nym_crypto::asymmetric::identity;
use nym_service_provider_directory_common::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    response::{ConfigResponse, PagedServicesListResponse},
    signing_types::construct_service_provider_announce_sign_payload,
    NymAddress, Service, ServiceDetails, ServiceId, ServiceType,
};
use rand_chacha::ChaCha20Rng;
use serde::de::DeserializeOwned;

use crate::test_helpers::{
    helpers::{get_app_attribute, nyms},
    signing::ed25519_sign_message,
};

use super::helpers::test_rng;

const DENOM: &str = "unym";
const ADDRESSES: &[&str] = &[
    "user",
    "admin",
    "announcer",
    "announcer1",
    "announcer2",
    "announcer3",
    "announcer4",
];
const WEALTHY_ADDRESSES: &[&str] = &["wealthy_announcer_1", "wealthy_announcer_2"];

// WIP(JON): consider moving this together with integration_tests

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

    // WIP(JON): remove all allow unused once done
    #[allow(unused)]
    pub fn address(&self) -> &Addr {
        &self.addr
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

    pub fn query_id(&self, service_id: ServiceId) -> Service {
        self.query(&QueryMsg::ServiceId { service_id })
    }

    pub fn query_all(&self) -> PagedServicesListResponse {
        self.query(&QueryMsg::all())
    }

    pub fn query_all_with_limit(
        &self,
        limit: Option<u32>,
        start_after: Option<u32>,
    ) -> PagedServicesListResponse {
        self.query(&QueryMsg::All { limit, start_after })
    }

    pub fn query_signing_nonce(&self, address: String) -> Nonce {
        self.query(&QueryMsg::SigningNonce { address })
    }

    pub fn announce_net_req(
        &mut self,
        nym_address: NymAddress,
        announcer: Addr,
    ) -> (AppResponse, identity::KeyPair) {
        let keypair = identity::KeyPair::new(&mut self.rng);

        // WIP(JON): add fn new
        let service = ServiceDetails {
            nym_address,
            service_type: ServiceType::NetworkRequester,
            identity_key: keypair.public_key().to_base58_string(),
        };

        let deposit = nyms(100);

        // Create payload, the same was as in announce_sign_payload
        let nonce = self.query_signing_nonce(announcer.to_string());
        println!("announcing: {announcer}");
        dbg!(&nonce);
        let payload_to_sign = construct_service_provider_announce_sign_payload(
            nonce,
            announcer.clone(),
            deposit,
            service.clone(),
        );

        // Now we sign it, like the user does manually
        let owner_signature = ed25519_sign_message(payload_to_sign, keypair.private_key());

        let resp = self
            .app
            .execute_contract(
                announcer,
                self.addr.clone(),
                &ExecuteMsg::Announce {
                    service,
                    owner_signature,
                },
                &[Coin {
                    denom: DENOM.to_string(),
                    amount: Uint128::new(100),
                }],
            )
            .unwrap();
        assert_eq!(
            get_app_attribute(&resp, "wasm-announce", "action"),
            "announce"
        );
        (resp, keypair)
    }

    pub fn try_delete(&mut self, service_id: ServiceId, announcer: Addr) -> Result<AppResponse> {
        self.app.execute_contract(
            announcer,
            self.addr.clone(),
            &ExecuteMsg::DeleteId { service_id },
            &[],
        )
    }

    pub fn delete(&mut self, service_id: ServiceId, announcer: Addr) -> AppResponse {
        let delete_resp = self.try_delete(service_id, announcer).unwrap();
        assert_eq!(
            get_app_attribute(&delete_resp, "wasm-delete_id", "action"),
            "delete_id"
        );
        delete_resp
    }

    pub fn delete_nym_address(&mut self, nym_address: NymAddress, announcer: Addr) -> AppResponse {
        self.app
            .execute_contract(
                announcer,
                self.addr.clone(),
                &ExecuteMsg::DeleteNymAddress { nym_address },
                &[],
            )
            .unwrap()
    }

    pub fn balance(&self, address: impl Into<String>) -> Coin {
        self.app.wrap().query_balance(address, DENOM).unwrap()
    }
}
