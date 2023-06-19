use cosmwasm_std::{from_binary, testing::mock_env, Addr, Coin, Deps};
use nym_contracts_common::signing::Nonce;
use nym_name_service_common::{
    msg::QueryMsg,
    response::{ConfigResponse, PagedNamesListResponse},
    NameId, RegisteredName,
};

use crate::{constants::NAME_DEFAULT_RETRIEVAL_LIMIT, NameServiceError};

pub fn assert_config(deps: Deps, admin: &Addr, deposit_required: Coin) {
    crate::state::assert_admin(deps, admin).unwrap();
    let res = crate::contract::query(deps, mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(config, ConfigResponse { deposit_required });
}

pub fn assert_names(deps: Deps, expected_names: &[RegisteredName]) {
    let res = crate::contract::query(deps, mock_env(), QueryMsg::all()).unwrap();
    let names: PagedNamesListResponse = from_binary(&res).unwrap();
    let start_next_after = expected_names.iter().last().map(|s| s.id);
    assert_eq!(
        names,
        PagedNamesListResponse {
            names: expected_names.to_vec(),
            per_page: NAME_DEFAULT_RETRIEVAL_LIMIT as usize,
            start_next_after,
        }
    );
}

pub fn assert_name(deps: Deps, expected_name: &RegisteredName) {
    let res = crate::contract::query(
        deps,
        mock_env(),
        QueryMsg::NameId {
            name_id: expected_name.id,
        },
    )
    .unwrap();
    let names: RegisteredName = from_binary(&res).unwrap();
    assert_eq!(&names, expected_name);
}

pub fn assert_empty(deps: Deps) {
    let res = crate::contract::query(deps, mock_env(), QueryMsg::all()).unwrap();
    let names: PagedNamesListResponse = from_binary(&res).unwrap();
    assert!(names.names.is_empty());
}

pub fn assert_not_found(deps: Deps, expected_id: NameId) {
    let res = crate::contract::query(
        deps,
        mock_env(),
        QueryMsg::NameId {
            name_id: expected_id,
        },
    )
    .unwrap_err();
    assert!(matches!(
        res,
        NameServiceError::NotFound {
            name_id: _expected_id
        }
    ));
}

pub fn assert_current_nonce(deps: Deps, address: &Addr, expected_nonce: Nonce) {
    let res = crate::contract::query(
        deps,
        mock_env(),
        QueryMsg::SigningNonce {
            address: address.to_string(),
        },
    )
    .unwrap();
    let nonce: Nonce = from_binary(&res).unwrap();
    assert_eq!(nonce, expected_nonce);
}
