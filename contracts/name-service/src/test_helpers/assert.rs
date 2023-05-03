use cosmwasm_std::{from_binary, testing::mock_env, Addr, Coin, Deps};
use nym_name_service_common::{
    msg::QueryMsg,
    response::{ConfigResponse, PagedNamesListResponse},
    NameEntry, NameId,
};

use crate::{constants::NAME_DEFAULT_RETRIEVAL_LIMIT, error::NameServiceError};

pub fn assert_config(deps: Deps, admin: &Addr, deposit_required: Coin) {
    crate::state::assert_admin(deps, admin).unwrap();
    let res = crate::contract::query(deps, mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(config, ConfigResponse { deposit_required });
}

pub fn assert_names(deps: Deps, expected_names: &[NameEntry]) {
    let res = crate::contract::query(deps, mock_env(), QueryMsg::all()).unwrap();
    let names: PagedNamesListResponse = from_binary(&res).unwrap();
    let start_next_after = expected_names.iter().last().map(|s| s.name_id);
    assert_eq!(
        names,
        PagedNamesListResponse {
            names: expected_names.to_vec(),
            per_page: NAME_DEFAULT_RETRIEVAL_LIMIT as usize,
            start_next_after,
        }
    );
}

pub fn assert_name(deps: Deps, expected_name: &NameEntry) {
    let res = crate::contract::query(
        deps,
        mock_env(),
        QueryMsg::NameId {
            name_id: expected_name.name_id,
        },
    )
    .unwrap();
    let names: NameEntry = from_binary(&res).unwrap();
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
