use cosmwasm_std::{
    coin,
    testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier},
    DepsMut, MemoryStorage, OwnedDeps,
};

use nym_service_provider_directory_common::{
    events::{ServiceProviderEventType, SERVICE_ID},
    msg::{ExecuteMsg, InstantiateMsg},
    ServiceDetails, ServiceId,
};
use rand_chacha::rand_core::{CryptoRng, RngCore};

use super::helpers::{get_attribute, nyms};

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

pub fn announce_service<R>(
    mut deps: DepsMut<'_>,
    rng: &mut R,
    nym_address: &str,
    announcer: &str,
) -> (ServiceId, ServiceDetails)
where
    R: RngCore + CryptoRng,
{
    let deposit = nyms(100);
    let (service, owner_signature) = super::fixture::new_service_details_with_sign(
        deps.branch(),
        rng,
        nym_address,
        announcer,
        deposit.clone(),
    );

    // Announce
    let msg = ExecuteMsg::Announce {
        service: service.clone(),
        owner_signature,
    };
    let info = mock_info(announcer, &[deposit]);
    let res = crate::execute(deps, mock_env(), info, msg).unwrap();

    let service_id: ServiceId = get_attribute(
        &res,
        &ServiceProviderEventType::Announce.to_string(),
        SERVICE_ID,
    )
    .parse()
    .unwrap();

    (service_id, service)
}

pub fn delete_service(deps: DepsMut<'_>, service_id: ServiceId, announcer: &str) {
    let msg = ExecuteMsg::DeleteId { service_id };
    let info = mock_info(announcer, &[]);
    crate::execute(deps, mock_env(), info, msg).unwrap();
}
