use cosmwasm_std::{
    coin, coins,
    testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier},
    Coin, DepsMut, Event, MemoryStorage, OwnedDeps, Response,
};
use cw_multi_test::AppResponse;
use nym_service_provider_directory_common::{
    events::{ServiceProviderEventType, SERVICE_ID},
    msg::{ExecuteMsg, InstantiateMsg},
    Service, ServiceId,
};

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

pub fn announce_service(deps: DepsMut<'_>, service: &Service) -> ServiceId {
    let msg: ExecuteMsg = service.clone().into();
    let info = mock_info(service.owner.as_str(), &coins(100, "unym"));
    let res = crate::execute(deps, mock_env(), info, msg).unwrap();
    let service_id: ServiceId = get_attribute(
        &res,
        &ServiceProviderEventType::Announce.to_string(),
        SERVICE_ID,
    )
    .parse()
    .unwrap();
    service_id
}

pub fn delete_service(deps: DepsMut<'_>, service_id: ServiceId, owner: &str) {
    let msg = ExecuteMsg::DeleteId { service_id };
    let info = mock_info(owner, &[]);
    crate::execute(deps, mock_env(), info, msg).unwrap();
}
