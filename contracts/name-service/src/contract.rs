use crate::{
    state::{self, Config},
    NameServiceError, Result,
};
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response};
use nym_name_service_common::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use semver::Version;

mod execute;
mod query;

// version info for migration info
const CONTRACT_NAME: &str = "crate:nym-name-service";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn instantiate(
    mut deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response> {
    state::set_admin(deps.branch(), info.sender.clone())?;

    let config = Config {
        deposit_required: msg.deposit_required,
    };
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    state::save_config(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", info.sender))
}

pub fn migrate(
    deps: DepsMut<'_>,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response, NameServiceError> {
    // Note: don't remove this particular bit of code as we have to ALWAYS check whether we have to
    // update the stored version
    let version: Version = CONTRACT_VERSION.parse().map_err(|error: semver::Error| {
        NameServiceError::SemVerFailure {
            value: CONTRACT_VERSION.to_string(),
            error_message: error.to_string(),
        }
    })?;

    let storage_version_raw = cw2::get_contract_version(deps.storage)?.version;
    let storage_version: Version =
        storage_version_raw
            .parse()
            .map_err(|error: semver::Error| NameServiceError::SemVerFailure {
                value: storage_version_raw,
                error_message: error.to_string(),
            })?;

    if storage_version < version {
        cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        // If state structure changed in any contract version in the way migration is needed, it
        // should occur here, for example anything from `crate::queued_migrations::`
    }

    Ok(Response::new())
}

pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, NameServiceError> {
    match msg {
        ExecuteMsg::Register {
            name,
            owner_signature,
        } => execute::register(deps, env, info, name, owner_signature),
        ExecuteMsg::DeleteId { name_id } => execute::delete_id(deps, info, name_id),
        ExecuteMsg::DeleteName { name } => execute::delete_name(deps, info, name),
        ExecuteMsg::UpdateDepositRequired { deposit_required } => {
            execute::update_deposit_required(deps, info, deposit_required)
        }
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary> {
    let response = match msg {
        QueryMsg::NameId { name_id } => to_binary(&query::query_id(deps, name_id)?),
        QueryMsg::ByOwner { owner } => to_binary(&query::query_owner(deps, owner)?),
        QueryMsg::ByAddress { address } => to_binary(&query::query_address(deps, address)?),
        QueryMsg::ByName { name } => to_binary(&query::query_name(deps, name)?),
        QueryMsg::All { limit, start_after } => {
            to_binary(&query::query_all_paged(deps, limit, start_after)?)
        }
        QueryMsg::SigningNonce { address } => {
            to_binary(&query::query_current_signing_nonce(deps, address)?)
        }
        QueryMsg::Config {} => to_binary(&query::query_config(deps)?),
        QueryMsg::GetContractVersion {} => to_binary(&query::query_contract_version()),
        QueryMsg::GetCW2ContractVersion {} => to_binary(&cw2::get_contract_version(deps.storage)?),
    };
    Ok(response?)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_helpers::{
        assert::{
            assert_config, assert_current_nonce, assert_empty, assert_name, assert_names,
            assert_not_found,
        },
        fixture::new_name_details_with_sign,
        helpers::{get_attribute, nyms, test_rng},
    };

    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Addr, Coin,
    };
    use nym_name_service_common::{NameId, RegisteredName};

    const DENOM: &str = "unym";

    #[test]
    fn instantiate_contract() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            deposit_required: Coin::new(100u128, DENOM),
        };
        let info = mock_info("creator", &[]);
        let admin = info.sender.clone();

        // Instantiate contract
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        // Check that it worked by querying the config, and checking that the list of names is
        // empty
        assert_config(deps.as_ref(), &admin, Coin::new(100u128, DENOM));
        assert_empty(deps.as_ref());
    }

    #[test]
    fn register_fails_deposit_too_small() {
        let mut rng = test_rng();
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg::new(nyms(100));
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        let deposit = nyms(99);
        let owner = "steve";
        let (name, owner_signature) =
            new_name_details_with_sign(deps.as_mut(), &mut rng, "foo", owner, deposit);
        let msg = ExecuteMsg::Register {
            name,
            owner_signature,
        };

        assert_eq!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(owner, &[nyms(99)]),
                msg.clone()
            )
            .unwrap_err(),
            NameServiceError::InsufficientDeposit {
                funds: 99u128.into(),
                deposit_required: 100u128.into(),
            }
        );

        // Since we signed for 99unym deposit.
        assert_eq!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(owner, &[nyms(100)]),
                msg
            )
            .unwrap_err(),
            NameServiceError::InvalidEd25519Signature,
        );
    }

    #[test]
    fn register_fails_deposit_too_large() {
        let mut rng = test_rng();
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg::new(nyms(100));
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        let deposit = nyms(101);
        let owner = "steve";
        let (name, owner_signature) =
            new_name_details_with_sign(deps.as_mut(), &mut rng, "foo", owner, deposit);
        let msg = ExecuteMsg::Register {
            name,
            owner_signature,
        };

        assert_eq!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(owner, &[nyms(101)]),
                msg.clone()
            )
            .unwrap_err(),
            NameServiceError::TooLargeDeposit {
                funds: 101u128.into(),
                deposit_required: 100u128.into(),
            }
        );

        // Since we signed for 101unym deposit.
        assert_eq!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(owner, &[nyms(100)]),
                msg
            )
            .unwrap_err(),
            NameServiceError::InvalidEd25519Signature,
        );
    }

    #[test]
    fn register_fails_owner_mismatch() {
        let mut rng = test_rng();
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg::new(nyms(100));
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        // Setup
        let deposit = nyms(100);
        let owner = "steve";
        let (name, owner_signature) =
            new_name_details_with_sign(deps.as_mut(), &mut rng, "my-name", owner, deposit);

        // Register
        let msg = ExecuteMsg::Register {
            name,
            owner_signature,
        };
        assert_eq!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info("timmy", &[nyms(100)]),
                msg.clone(),
            )
            .unwrap_err(),
            NameServiceError::InvalidEd25519Signature,
        );
        assert!(execute(
            deps.as_mut(),
            mock_env(),
            mock_info("steve", &[nyms(100)]),
            msg
        )
        .is_ok());
    }

    #[test]
    fn register_success() {
        let mut rng = test_rng();
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg::new(nyms(100));
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        // Setup
        let deposit = nyms(100);
        let owner = "steve";
        let (name, owner_signature) =
            new_name_details_with_sign(deps.as_mut(), &mut rng, "my-name", owner, deposit.clone());

        // Register
        let msg = ExecuteMsg::Register {
            name: name.clone(),
            owner_signature,
        };
        let info = mock_info("steve", &[nyms(100)]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Check that the name has had name id assigned to it
        let expected_id = 1;
        let id: NameId = get_attribute(&res, "register", "name_id").parse().unwrap();
        assert_eq!(id, expected_id);
        assert_eq!(
            get_attribute(&res, "register", "name"),
            "my-name".to_string()
        );

        // Check that the nonce has been incremented, but only for the owner
        assert_current_nonce(deps.as_ref(), &Addr::unchecked("steve"), 1);
        assert_current_nonce(deps.as_ref(), &Addr::unchecked("timmy"), 0);

        // The expected registered name
        let expected_name = RegisteredName {
            id: expected_id,
            name,
            owner: Addr::unchecked(owner),
            block_height: 12345,
            deposit,
        };
        assert_names(deps.as_ref(), &[expected_name.clone()]);
        assert_name(deps.as_ref(), &expected_name);
    }

    #[test]
    fn delete() {
        let mut rng = test_rng();
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg::new(Coin::new(100, "unym"));
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        // Register
        let deposit = nyms(100);
        let steve = "steve";
        let (name, owner_signature) =
            new_name_details_with_sign(deps.as_mut(), &mut rng, "my-name", steve, deposit.clone());
        let msg = ExecuteMsg::Register {
            name: name.clone(),
            owner_signature,
        };
        let info_steve = mock_info("steve", &[nyms(100)]);
        execute(deps.as_mut(), mock_env(), info_steve.clone(), msg).unwrap();

        // The expected registerd name
        let expected_id = 1;
        let expected_name = RegisteredName {
            id: expected_id,
            name,
            owner: Addr::unchecked(steve),
            block_height: 12345,
            deposit,
        };
        assert_names(deps.as_ref(), &[expected_name]);

        // Removing someone else's name will fail
        let msg = ExecuteMsg::delete_id(expected_id);
        let info_timmy = mock_info("timmy", &[]);
        assert_eq!(
            execute(deps.as_mut(), mock_env(), info_timmy, msg).unwrap_err(),
            NameServiceError::Unauthorized {
                sender: Addr::unchecked("timmy")
            }
        );

        // Removing an non-existent name will fail
        let msg = ExecuteMsg::delete_id(expected_id + 1);
        assert_eq!(
            execute(deps.as_mut(), mock_env(), info_steve.clone(), msg).unwrap_err(),
            NameServiceError::NotFound {
                name_id: expected_id + 1
            }
        );

        // Remove as correct owner succeeds
        let msg = ExecuteMsg::delete_id(expected_id);
        let res = execute(deps.as_mut(), mock_env(), info_steve, msg).unwrap();
        assert_eq!(
            get_attribute(&res, "delete_id", "name_id"),
            expected_id.to_string()
        );
        assert_names(deps.as_ref(), &[]);
        assert_not_found(deps.as_ref(), expected_id);
    }
}
