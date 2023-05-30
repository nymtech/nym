use cosmwasm_std::{
    coin, coins,
    testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier},
    Coin, DepsMut, Event, MemoryStorage, OwnedDeps, Response, Addr, Deps,
};
use cw_multi_test::AppResponse;
use nym_contracts_common::signing::{SigningPurpose, SignableMessage, MessageSignature, SigningAlgorithm};
use nym_crypto::asymmetric::identity;
use nym_service_provider_directory_common::{
    events::{ServiceProviderEventType, SERVICE_ID},
    msg::{ExecuteMsg, InstantiateMsg},
    Service, ServiceDetails, ServiceId, signing_types::{construct_service_provider_announce_sign_payload, SignableServiceProviderAnnounceMsg},
};
use serde::Serialize;

use crate::signing;

pub fn nyms(amount: u64) -> Coin {
    Coin::new(amount.into(), "unym")
}

pub fn get_event_types(response: &Response, event_type: &str) -> Vec<Event> {
    response
        .events
        .iter()
        .filter(|ev| ev.ty == event_type)
        .cloned()
        .collect()
}

pub fn get_attribute(response: &Response, event_type: &str, key: &str) -> String {
    get_event_types(response, event_type)
        .first()
        .unwrap()
        .attributes
        .iter()
        .find(|attr| attr.key == key)
        .unwrap()
        .value
        .clone()
}

pub fn get_app_event_types(response: &AppResponse, event_type: &str) -> Vec<Event> {
    response
        .events
        .iter()
        .filter(|ev| ev.ty == event_type)
        .cloned()
        .collect()
}

pub fn get_app_attribute(response: &AppResponse, event_type: &str, key: &str) -> String {
    get_app_event_types(response, event_type)
        .first()
        .unwrap()
        .attributes
        .iter()
        .find(|attr| attr.key == key)
        .unwrap()
        .value
        .clone()
}

#[allow(dead_code)]
pub fn get_app_attributes(response: &AppResponse, event_type: &str, key: &str) -> Vec<String> {
    get_app_event_types(response, event_type)
        .iter()
        .map(|ev| {
            ev.attributes
                .iter()
                .find(|attr| attr.key == key)
                .unwrap()
                .value
                .clone()
        })
        .collect::<Vec<_>>()
}

pub fn instantiate_test_contract() -> OwnedDeps<MemoryStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies();
    let msg = InstantiateMsg {
        deposit_required: coin(100, "unym"),
    };
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let res = crate::instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(res.messages.len(), 0);
    deps
}

//use nym_crypto::asymmetric::identity;

//pub fn announce_service(deps: DepsMut<'_>, service: &ServiceDetails, announcer: &str) -> ServiceId {
//    let keypair = identity::KeyPair::new();
//    let identity_key = keypair.public_key().to_base58_string();
//
//    let msg = service_provider_announce_sign_payload(deps, sender, service.clone(), deposit);
//
//    //let msg: ExecuteMsg = service.clone().into();
//    let msg = ExecuteMsg::Announce {
//        service: service.clone(),
//        owner_signature: todo!(),
//    };
//    //let info = mock_info(service.announcer.as_str(), &coins(100, "unym"));
//    let info = mock_info(announcer, &coins(100, "unym"));
//    let res = crate::execute(deps, mock_env(), info, msg).unwrap();
//    let service_id: ServiceId = get_attribute(
//        &res,
//        &ServiceProviderEventType::Announce.to_string(),
//        SERVICE_ID,
//    )
//    .parse()
//    .unwrap();
//    service_id
//}

pub fn delete_service(deps: DepsMut<'_>, service_id: ServiceId, announcer: &str) {
    let msg = ExecuteMsg::DeleteId { service_id };
    let info = mock_info(announcer, &[]);
    crate::execute(deps, mock_env(), info, msg).unwrap();
}

//
// Signing
//

pub fn service_provider_announce_sign_payload(
    deps: Deps<'_>,
    owner: &str,
    service: ServiceDetails,
    deposit: Coin,
) -> SignableServiceProviderAnnounceMsg {
    let owner = Addr::unchecked(owner);
    let nonce = signing::storage::get_signing_nonce(deps.storage, owner.clone()).unwrap();
    construct_service_provider_announce_sign_payload(nonce, owner, deposit, service)
}

pub fn ed25519_sign_message<T: Serialize + SigningPurpose>(
    message: SignableMessage<T>,
    private_key: &identity::PrivateKey,
) -> MessageSignature {
    match message.algorithm {
        SigningAlgorithm::Ed25519 => {
            let plaintext = message.to_plaintext().unwrap();
            let signature = private_key.sign(&plaintext);
            MessageSignature::from(signature.to_bytes().as_ref())
        }
        SigningAlgorithm::Secp256k1 => {
            unimplemented!()
        }
    }
}
