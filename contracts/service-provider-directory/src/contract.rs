use crate::{
    error::{ContractError, Result},
    state::{self, Config},
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

pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Note: don't remove this particular bit of code as we have to ALWAYS check whether we have to
    // update the stored version
    let version: Version =
        CONTRACT_VERSION
            .parse()
            .map_err(|error: semver::Error| ContractError::SemVerFailure {
                value: CONTRACT_VERSION.to_string(),
                error_message: error.to_string(),
            })?;

    let storage_version_raw = cw2::get_contract_version(deps.storage)?.version;
    let storage_version: Version =
        storage_version_raw
            .parse()
            .map_err(|error: semver::Error| ContractError::SemVerFailure {
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
) -> Result<Response, ContractError> {
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
        QueryMsg::Config {} => to_binary(&query::query_config(deps)?),
        QueryMsg::GetContractVersion {} => to_binary(&query::query_contract_version()),
        QueryMsg::GetCW2ContractVersion {} => to_binary(&cw2::get_contract_version(deps.storage)?),
    };
    Ok(response?)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        signing,
        test_helpers::{
            assert::{
                assert_config, assert_empty, assert_not_found, assert_service, assert_services,
            },
            fixture::service_fixture,
            helpers::{get_attribute, nyms},
        },
    };

    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Addr, Coin,
    };
    use nym_contracts_common::signing::{
        ContractMessageContent, MessageSignature, SignableMessage, SigningAlgorithm, SigningPurpose,
    };
    use nym_crypto::asymmetric::identity;
    use nym_service_provider_directory_common::{
        msg::ExecuteMsg,
        signing_types::{
            construct_service_provider_announce_sign_payload, ServiceProviderAnnounce,
            SignableServiceProviderAnnounceMsg,
        },
        NymAddress, Service, ServiceDetails, ServiceId, ServiceType,
    };
    use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
    use serde::Serialize;

    const DENOM: &str = "unym";

    // WIP(JON): move to test helpers
    fn test_rng() -> ChaCha20Rng {
        let dummy_seed = [42u8; 32];
        ChaCha20Rng::from_seed(dummy_seed)
    }

    fn service_provider_announce_sign_payload(
        deps: Deps<'_>,
        owner: &str,
        service: ServiceDetails,
        deposit: Coin,
    ) -> SignableServiceProviderAnnounceMsg {
        let owner = Addr::unchecked(owner);
        let nonce = signing::storage::get_signing_nonce(deps.storage, owner.clone()).unwrap();
        construct_service_provider_announce_sign_payload(nonce, owner, deposit, service)
    }

    fn ed25519_sign_message<T: Serialize + SigningPurpose>(
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

    //#[test]
    //fn announce_fails_incorrect_deposit() {
    //    let mut deps = mock_dependencies();
    //    let msg = InstantiateMsg::new(nyms(100));
    //    let info = mock_info("creator", &[]);
    //    let admin = info.sender.clone();
    //    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    //    assert_eq!(res.messages.len(), 0);

    //    // Announce
    //    let msg: ExecuteMsg = service_fixture().into();
    //    let announcer = service_fixture().announcer.to_string();

    //    assert_eq!(
    //        execute(
    //            deps.as_mut(),
    //            mock_env(),
    //            mock_info(&announcer, &[nyms(99)]),
    //            msg.clone()
    //        )
    //        .unwrap_err(),
    //        ContractError::InsufficientDeposit {
    //            funds: 99u128.into(),
    //            deposit_required: 100u128.into(),
    //        }
    //    );

    //    assert_eq!(
    //        execute(
    //            deps.as_mut(),
    //            mock_env(),
    //            mock_info(&announcer, &[nyms(101)]),
    //            msg
    //        )
    //        .unwrap_err(),
    //        ContractError::TooLargeDeposit {
    //            funds: 101u128.into(),
    //            deposit_required: 100u128.into(),
    //        }
    //    );

    //    assert_config(deps.as_ref(), &admin, Coin::new(100, DENOM));
    //    assert_empty(deps.as_ref());
    //}

    #[test]
    fn announce_success() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg::new(nyms(100));
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        // Setup service
        let mut rng = test_rng();
        let keypair = identity::KeyPair::new(&mut rng);
        let identity_key = keypair.public_key().to_base58_string();
        let service = ServiceDetails {
            nym_address: NymAddress::new("nym"),
            service_type: ServiceType::NetworkRequester,
            identity_key,
        };
        let announcer = "steve";
        let deposit = nyms(100);

        // Sign
        let sign_msg = service_provider_announce_sign_payload(
            deps.as_ref(),
            announcer,
            service.clone(),
            deposit.clone(),
        );
        let owner_signature = ed25519_sign_message(sign_msg, keypair.private_key());

        // Announce
        let msg = ExecuteMsg::Announce {
            service: service.clone(),
            owner_signature,
        };
        let info = mock_info("steve", &[nyms(100)]);
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

    //#[test]
    //fn delete() {
    //    let mut deps = mock_dependencies();
    //    let msg = InstantiateMsg::new(Coin::new(100, "unym"));
    //    let info = mock_info("creator", &[]);
    //    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    //    assert_eq!(res.messages.len(), 0);

    //    // Announce
    //    let msg: ExecuteMsg = service_fixture().into();
    //    let info_steve = mock_info("steve", &[nyms(100)]);
    //    assert_eq!(info_steve.sender, service_fixture().announcer);
    //    execute(deps.as_mut(), mock_env(), info_steve, msg).unwrap();

    //    // The expected announced service
    //    let expected_id = 1;
    //    let expected_service = ServiceInfo {
    //        service_id: expected_id,
    //        service: service_fixture(),
    //    };
    //    assert_services(deps.as_ref(), &[expected_service]);

    //    // Removing someone else's service will fail
    //    let msg = ExecuteMsg::delete_id(expected_id);
    //    let info_timmy = mock_info("timmy", &[]);
    //    assert_eq!(
    //        execute(deps.as_mut(), mock_env(), info_timmy, msg).unwrap_err(),
    //        ContractError::Unauthorized {
    //            sender: Addr::unchecked("timmy")
    //        }
    //    );

    //    // Removing an non-existent service will fail
    //    let msg = ExecuteMsg::delete_id(expected_id + 1);
    //    let info_announcer = MessageInfo {
    //        sender: service_fixture().announcer,
    //        funds: vec![],
    //    };
    //    assert_eq!(
    //        execute(deps.as_mut(), mock_env(), info_announcer.clone(), msg).unwrap_err(),
    //        ContractError::NotFound {
    //            service_id: expected_id + 1
    //        }
    //    );

    //    // Remove as correct announcer succeeds
    //    let msg = ExecuteMsg::delete_id(expected_id);
    //    let res = execute(deps.as_mut(), mock_env(), info_announcer, msg).unwrap();
    //    assert_eq!(
    //        get_attribute(&res, "delete_id", "service_id"),
    //        expected_id.to_string()
    //    );
    //    assert_services(deps.as_ref(), &[]);
    //    assert_not_found(deps.as_ref(), expected_id);
    //}
}
