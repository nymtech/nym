use anyhow::Result;
use cosmwasm_std::{coins, Addr, Coin, Uint128};
use cw_multi_test::{App, AppBuilder, AppResponse, ContractWrapper, Executor};
use nym_contracts_common::signing::Nonce;
use nym_service_provider_directory_common::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    response::{ConfigResponse, PagedServicesListResponse},
    signing_types::{
        construct_service_provider_announce_sign_payload, SignableServiceProviderAnnounceMsg,
    },
    NymAddress, Service, ServiceDetails, ServiceId,
};
use rand_chacha::ChaCha20Rng;
use serde::de::DeserializeOwned;

use crate::test_helpers::helpers::{get_app_attribute, test_rng};

use super::test_service::{SignedTestService, TestService};

const DENOM: &str = "unym";
const ADDRESSES: &[&str] = &[
    "user",
    "admin",
    "announcer",
    "announcer1",
    "announcer2",
    "announcer3",
    "announcer4",
    "steve",
    "timmy",
];
const WEALTHY_ADDRESSES: &[&str] = &["wealthy_announcer_1", "wealthy_announcer_2"];

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

    // Create a new service, together with its signing keypair
    pub fn new_service(&mut self, nym_address: &NymAddress) -> TestService {
        TestService::new(&mut self.rng, nym_address.clone())
    }

    // Create payload for the service operator to sign
    pub fn payload_to_sign(
        &mut self,
        announcer: &Addr,
        deposit: &Coin,
        service: &ServiceDetails,
    ) -> SignableServiceProviderAnnounceMsg {
        let nonce = self.query_signing_nonce(announcer.to_string());
        construct_service_provider_announce_sign_payload(
            nonce,
            announcer.clone(),
            deposit.clone(),
            service.clone(),
        )
    }

    // Convenience function for creating a new service and signing it.
    pub fn new_signed_service(
        &mut self,
        nym_address: &NymAddress,
        announcer: &Addr,
        deposit: &Coin,
    ) -> SignedTestService {
        let service = self.new_service(nym_address);
        let payload = self.payload_to_sign(announcer, deposit, service.details());
        service.sign(payload)
    }

    // Announce a new service
    pub fn try_announce_net_req(
        &mut self,
        service: &SignedTestService,
        announcer: &Addr,
    ) -> Result<AppResponse> {
        self.app.execute_contract(
            announcer.clone(),
            self.addr.clone(),
            &ExecuteMsg::Announce {
                service: service.service.clone(),
                owner_signature: service.owner_signature.clone(),
            },
            &[Coin {
                denom: DENOM.to_string(),
                amount: Uint128::new(100),
            }],
        )
    }

    pub fn announce_net_req(
        &mut self,
        service: &SignedTestService,
        announcer: &Addr,
    ) -> AppResponse {
        let resp = self.try_announce_net_req(service, announcer).unwrap();
        assert_eq!(
            get_app_attribute(&resp, "wasm-announce", "action"),
            "announce"
        );
        resp
    }

    // Convenience function for create a new signed service and announcing it
    pub fn sign_and_announce_net_req(
        &mut self,
        nym_address: &NymAddress,
        announcer: &Addr,
        deposit: &Coin,
    ) -> SignedTestService {
        let service = self.new_signed_service(nym_address, announcer, deposit);
        let _ = self.announce_net_req(&service, announcer);
        service
    }

    pub fn try_delete(&mut self, service_id: ServiceId, announcer: &Addr) -> Result<AppResponse> {
        self.app.execute_contract(
            announcer.clone(),
            self.addr.clone(),
            &ExecuteMsg::DeleteId { service_id },
            &[],
        )
    }

    pub fn delete(&mut self, service_id: ServiceId, announcer: &Addr) -> AppResponse {
        let delete_resp = self.try_delete(service_id, announcer).unwrap();
        assert_eq!(
            get_app_attribute(&delete_resp, "wasm-delete_id", "action"),
            "delete_id"
        );
        delete_resp
    }

    pub fn delete_nym_address(
        &mut self,
        nym_address: &NymAddress,
        announcer: &Addr,
    ) -> AppResponse {
        self.app
            .execute_contract(
                announcer.clone(),
                self.addr.clone(),
                &ExecuteMsg::DeleteNymAddress {
                    nym_address: nym_address.clone(),
                },
                &[],
            )
            .unwrap()
    }

    pub fn balance(&self, address: impl Into<String>) -> Coin {
        self.app.wrap().query_balance(address, DENOM).unwrap()
    }
}
