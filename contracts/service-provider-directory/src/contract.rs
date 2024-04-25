use crate::{
    state::{self, Config},
    Result, SpContractError,
};
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response};
use nym_service_provider_directory_common::msg::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use semver::Version;

mod execute;
mod query;

// version info for migration info
const CONTRACT_NAME: &str = "crate:nym-service-provider-directory";
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
) -> Result<Response, SpContractError> {
    // Note: don't remove this particular bit of code as we have to ALWAYS check whether we have to
    // update the stored version
    let version: Version = CONTRACT_VERSION.parse().map_err(|error: semver::Error| {
        SpContractError::SemVerFailure {
            value: CONTRACT_VERSION.to_string(),
            error_message: error.to_string(),
        }
    })?;

    let storage_version_raw = cw2::get_contract_version(deps.storage)?.version;
    let storage_version: Version =
        storage_version_raw
            .parse()
            .map_err(|error: semver::Error| SpContractError::SemVerFailure {
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
) -> Result<Response, SpContractError> {
    match msg {
        ExecuteMsg::Announce {
            service,
            owner_signature,
        } => execute::announce(deps, env, info, service, owner_signature),
        ExecuteMsg::DeleteId { service_id } => execute::delete_id(deps, info, service_id),
        ExecuteMsg::DeleteNymAddress { nym_address } => {
            execute::delete_nym_address(deps, info, nym_address)
        }
        ExecuteMsg::UpdateDepositRequired { deposit_required } => {
            execute::update_deposit_required(deps, info, deposit_required)
        }
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary> {
    let response = match msg {
        QueryMsg::ServiceId { service_id } => to_binary(&query::query_id(deps, service_id)?),
        QueryMsg::ByAnnouncer { announcer } => to_binary(&query::query_announcer(deps, announcer)?),
        QueryMsg::ByNymAddress { nym_address } => {
            to_binary(&query::query_nym_address(deps, nym_address)?)
        }
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
            assert_config, assert_current_nonce, assert_empty, assert_not_found, assert_service,
            assert_services,
        },
        fixture::new_service_details_with_sign,
        helpers::{get_attribute, nyms, test_rng},
    };

    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Addr, Coin,
    };
    use nym_service_provider_directory_common::{Service, ServiceId};

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

        // Check that it worked by querying the config, and checking that the list of services is
        // empty
        assert_config(deps.as_ref(), &admin, Coin::new(100u128, DENOM));
        assert_empty(deps.as_ref());
    }

    #[test]
    fn announce_fails_incorrect_deposit_too_small() {
        let mut rng = test_rng();
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg::new(nyms(100));
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        // Setup service
        let deposit = nyms(99);
        let announcer = "steve";
        let (service, owner_signature) =
            new_service_details_with_sign(deps.as_mut(), &mut rng, "nym", announcer, deposit);
        let msg = ExecuteMsg::Announce {
            service,
            owner_signature,
        };

        assert_eq!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(announcer, &[nyms(99)]),
                msg.clone()
            )
            .unwrap_err(),
            SpContractError::InsufficientDeposit {
                funds: 99u128.into(),
                deposit_required: 100u128.into(),
            }
        );

        assert_eq!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(announcer, &[nyms(100)]),
                msg,
            )
            .unwrap_err(),
            SpContractError::InvalidEd25519Signature,
        );
    }

    // Announcing a service fails due to the signed deposit being different from the deposit in
    // the message.
    #[test]
    fn announce_fails_incorrect_deposit_too_large() {
        let mut rng = test_rng();
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg::new(nyms(100));
        let info = mock_info("creator", &[]);
        let admin = info.sender.clone();
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        // Setup service
        let deposit = nyms(101);
        let announcer = "steve";
        let (service, owner_signature) =
            new_service_details_with_sign(deps.as_mut(), &mut rng, "nym", announcer, deposit);
        let msg = ExecuteMsg::Announce {
            service,
            owner_signature,
        };

        assert_eq!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(announcer, &[nyms(100)]),
                msg.clone()
            )
            .unwrap_err(),
            SpContractError::InvalidEd25519Signature,
        );
        assert_eq!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(announcer, &[nyms(101)]),
                msg,
            )
            .unwrap_err(),
            SpContractError::TooLargeDeposit {
                funds: 101u128.into(),
                deposit_required: 100u128.into(),
            }
        );

        assert_config(deps.as_ref(), &admin, Coin::new(100, DENOM));
        assert_empty(deps.as_ref());
    }

    #[test]
    fn announce_success() {
        let mut rng = test_rng();
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg::new(nyms(100));
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);
        assert_current_nonce(deps.as_ref(), &Addr::unchecked("steve"), 0);

        // Setup service
        let deposit = nyms(100);
        let owner = "steve";
        let (service, owner_signature) =
            new_service_details_with_sign(deps.as_mut(), &mut rng, "nym", owner, deposit.clone());

        // Announce
        let msg = ExecuteMsg::Announce {
            service: service.clone(),
            owner_signature,
        };
        let info = mock_info("steve", &[deposit.clone()]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Check that the service has had service id assigned to it
        let expected_id = 1;
        let id: ServiceId = get_attribute(&res, "announce", "service_id")
            .parse()
            .unwrap();
        assert_eq!(id, expected_id);
        assert_eq!(
            get_attribute(&res, "announce", "service_type"),
            "network_requester".to_string()
        );

        // Check that the nonce has been incremented, but only for the owner
        assert_current_nonce(deps.as_ref(), &Addr::unchecked("steve"), 1);
        assert_current_nonce(deps.as_ref(), &Addr::unchecked("timmy"), 0);

        // The expected announced service
        let expected_service = Service {
            service_id: expected_id,
            service,
            announcer: Addr::unchecked("steve"),
            block_height: 12345,
            deposit,
        };
        assert_services(deps.as_ref(), &[expected_service.clone()]);
        assert_service(deps.as_ref(), &expected_service);
    }

    #[test]
    fn delete() {
        let mut rng = test_rng();
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg::new(Coin::new(100, "unym"));
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        // Announce
        let deposit = nyms(100);
        let steve = "steve";
        let (service, owner_signature) =
            new_service_details_with_sign(deps.as_mut(), &mut rng, "nym", steve, deposit.clone());
        let msg = ExecuteMsg::Announce {
            service: service.clone(),
            owner_signature,
        };
        let info_steve = mock_info(steve, &[deposit.clone()]);
        execute(deps.as_mut(), mock_env(), info_steve.clone(), msg).unwrap();

        // The expected announced service
        let expected_id = 1;
        let expected_service = Service {
            service_id: expected_id,
            service,
            announcer: Addr::unchecked(steve),
            block_height: 12345,
            deposit,
        };
        assert_services(deps.as_ref(), &[expected_service]);

        // Removing someone else's service will fail
        let msg = ExecuteMsg::delete_id(expected_id);
        let info_timmy = mock_info("timmy", &[]);
        assert_eq!(
            execute(deps.as_mut(), mock_env(), info_timmy, msg).unwrap_err(),
            SpContractError::Unauthorized {
                sender: Addr::unchecked("timmy")
            }
        );

        // Removing an non-existent service will fail
        let msg = ExecuteMsg::delete_id(expected_id + 1);
        assert_eq!(
            execute(deps.as_mut(), mock_env(), info_steve.clone(), msg).unwrap_err(),
            SpContractError::NotFound {
                service_id: expected_id + 1
            }
        );

        // Remove as correct announcer succeeds
        let msg = ExecuteMsg::delete_id(expected_id);
        let res = execute(deps.as_mut(), mock_env(), info_steve, msg).unwrap();
        assert_eq!(
            get_attribute(&res, "delete_id", "service_id"),
            expected_id.to_string()
        );
        assert_services(deps.as_ref(), &[]);
        assert_not_found(deps.as_ref(), expected_id);
    }
}
