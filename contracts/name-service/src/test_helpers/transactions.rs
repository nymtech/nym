use cosmwasm_std::{
    coin,
    testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier},
    DepsMut, MemoryStorage, OwnedDeps,
};
use nym_name_service_common::{
    events::{NameEventType, NAME_ID},
    msg::{ExecuteMsg, InstantiateMsg},
    NameDetails, NameId, NymName,
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

pub fn register_name<R>(
    mut deps: DepsMut<'_>,
    rng: &mut R,
    name: &str,
    owner: &str,
) -> (NameId, NameDetails)
where
    R: RngCore + CryptoRng,
{
    let deposit = nyms(100);
    let (name, owner_signature) = super::fixture::new_name_details_with_sign(
        deps.branch(),
        rng,
        name,
        owner,
        deposit.clone(),
    );

    // Register
    let msg = ExecuteMsg::Register {
        name: name.clone(),
        owner_signature,
    };
    let info = mock_info(owner, &[deposit]);
    let res = crate::execute(deps, mock_env(), info, msg).unwrap();

    let name_id: NameId = get_attribute(&res, &NameEventType::Register.to_string(), NAME_ID)
        .parse()
        .unwrap();
    (name_id, name)
}

pub fn delete_name_id(deps: DepsMut<'_>, name_id: NameId, owner: &str) {
    let msg = ExecuteMsg::DeleteId { name_id };
    let info = mock_info(owner, &[]);
    crate::execute(deps, mock_env(), info, msg).unwrap();
}

#[allow(unused)]
pub fn delete_name(deps: DepsMut<'_>, name: NymName, owner: &str) {
    let msg = ExecuteMsg::DeleteName { name };
    let info = mock_info(owner, &[]);
    crate::execute(deps, mock_env(), info, msg).unwrap();
}
