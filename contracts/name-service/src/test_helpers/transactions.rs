use cosmwasm_std::{
    coin, coins,
    testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier},
    DepsMut, MemoryStorage, OwnedDeps,
};
use nym_name_service_common::{
    events::{NameEventType, NAME_ID},
    msg::{ExecuteMsg, InstantiateMsg},
    NameId, NymName, RegisteredName,
};

use super::helpers::get_attribute;

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

pub fn register_name(deps: DepsMut<'_>, name: &RegisteredName) -> NameId {
    let msg: ExecuteMsg = name.clone().into();
    let info = mock_info(name.owner.as_str(), &coins(100, "unym"));
    let res = crate::execute(deps, mock_env(), info, msg).unwrap();
    let name_id: NameId = get_attribute(&res, &NameEventType::Register.to_string(), NAME_ID)
        .parse()
        .unwrap();
    name_id
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
