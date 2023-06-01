use crate::{
    constants::{MAX_NUMBER_OF_ALIASES_FOR_NYM_ADDRESS, MAX_NUMBER_OF_PROVIDERS_PER_ANNOUNCER},
    state, Result, SpContractError,
};
use cosmwasm_std::{Addr, BankMsg, Coin, Deps, DepsMut, Env, MessageInfo, Response, Uint128};
use nym_contracts_common::signing::{MessageSignature, Verifier};
use nym_service_provider_directory_common::{
    events::{new_announce_event, new_delete_id_event, new_update_deposit_required_event},
    signing_types::construct_service_provider_announce_sign_payload,
    IdentityKey, NymAddress, Service, ServiceDetails, ServiceId,
};

use super::query;

fn ensure_correct_deposit(will_deposit: Uint128, deposit_required: Uint128) -> Result<()> {
    match will_deposit.cmp(&deposit_required) {
        std::cmp::Ordering::Less => Err(SpContractError::InsufficientDeposit {
            funds: will_deposit,
            deposit_required,
        }),
        std::cmp::Ordering::Equal => Ok(()),
        std::cmp::Ordering::Greater => Err(SpContractError::TooLargeDeposit {
            funds: will_deposit,
            deposit_required,
        }),
    }
}

fn ensure_max_services_per_announcer(deps: Deps, announcer: Addr) -> Result<()> {
    let current_entries = query::query_announcer(deps, announcer.to_string())?;
    if current_entries.services.len() < MAX_NUMBER_OF_PROVIDERS_PER_ANNOUNCER as usize {
        Ok(())
    } else {
        Err(SpContractError::ReachedMaxProvidersForAdmin {
            max_providers: MAX_NUMBER_OF_PROVIDERS_PER_ANNOUNCER,
            announcer,
        })
    }
}

fn ensure_max_aliases_per_nym_address(deps: Deps, nym_address: NymAddress) -> Result<()> {
    let current_entries = query::query_nym_address(deps, nym_address.clone())?;
    if current_entries.services.len() < MAX_NUMBER_OF_ALIASES_FOR_NYM_ADDRESS as usize {
        Ok(())
    } else {
        Err(SpContractError::ReachedMaxAliasesForNymAddress {
            max_aliases: MAX_NUMBER_OF_ALIASES_FOR_NYM_ADDRESS,
            nym_address,
        })
    }
}

fn ensure_service_exists(deps: Deps, service_id: ServiceId) -> Result<()> {
    if state::has_service(deps.storage, service_id) {
        Ok(())
    } else {
        Err(SpContractError::NotFound { service_id })
    }
}

fn ensure_sender_authorized(info: MessageInfo, service: &Service) -> Result<()> {
    if info.sender == service.announcer {
        Ok(())
    } else {
        Err(SpContractError::Unauthorized {
            sender: info.sender,
        })
    }
}

fn return_deposit(service_to_delete: &Service) -> BankMsg {
    BankMsg::Send {
        to_address: service_to_delete.announcer.to_string(),
        amount: vec![service_to_delete.deposit.clone()],
    }
}

fn verify_announce_signature(
    deps: Deps<'_>,
    sender: Addr,
    deposit: Coin,
    service: ServiceDetails,
    signature: MessageSignature,
) -> Result<()> {
    // recover the public key
    let public_key = decode_ed25519_identity_key(&service.identity_key)?;

    // reconstruct the payload
    let nonce = state::get_signing_nonce(deps.storage, sender.clone())?;

    let msg = construct_service_provider_announce_sign_payload(nonce, sender, deposit, service);

    if deps.api.verify_message(msg, signature, &public_key)? {
        Ok(())
    } else {
        Err(SpContractError::InvalidEd25519Signature)
    }
}

fn decode_ed25519_identity_key(encoded: &IdentityKey) -> Result<[u8; 32]> {
    let mut public_key = [0u8; 32];
    let used = bs58::decode(encoded)
        .into(&mut public_key)
        .map_err(|err| SpContractError::MalformedEd25519IdentityKey(err.to_string()))?;

    if used != 32 {
        return Err(SpContractError::MalformedEd25519IdentityKey(
            "Too few bytes provided for the public key".into(),
        ));
    }

    Ok(public_key)
}

/// Announce a new service. It will be assigned a new service provider id.
pub fn announce(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    service: ServiceDetails,
    owner_signature: MessageSignature,
) -> Result<Response> {
    ensure_max_services_per_announcer(deps.as_ref(), info.sender.clone())?;
    ensure_max_aliases_per_nym_address(deps.as_ref(), service.nym_address.clone())?;

    let deposit_required = state::deposit_required(deps.storage)?;
    let denom = deposit_required.denom.clone();
    let will_deposit = cw_utils::must_pay(&info, &denom)
        .map_err(|err| SpContractError::DepositRequired { source: err })?;
    ensure_correct_deposit(will_deposit, deposit_required.amount)?;

    let deposit = Coin::new(will_deposit.u128(), denom);

    // Check that the sender actually owns the service provider by checking the signature
    verify_announce_signature(
        deps.as_ref(),
        info.sender.clone(),
        deposit.clone(),
        service.clone(),
        owner_signature,
    )?;

    state::increment_signing_nonce(deps.storage, info.sender.clone())?;

    let service_id = state::next_service_id_counter(deps.storage)?;
    let new_service = Service {
        service_id,
        service,
        announcer: info.sender,
        block_height: env.block.height,
        deposit,
    };
    state::save(deps.storage, &new_service)?;

    Ok(Response::new().add_event(new_announce_event(service_id, new_service)))
}

/// Delete an exsisting service.
pub fn delete_id(deps: DepsMut, info: MessageInfo, service_id: ServiceId) -> Result<Response> {
    ensure_service_exists(deps.as_ref(), service_id)?;
    let service_to_delete = state::load_id(deps.storage, service_id)?;
    ensure_sender_authorized(info, &service_to_delete)?;

    state::remove(deps.storage, service_id)?;
    let return_deposit_msg = return_deposit(&service_to_delete);

    Ok(Response::new()
        .add_message(return_deposit_msg)
        .add_event(new_delete_id_event(service_to_delete)))
}

/// Delete an existing service by nym address. If there are multiple entries for a given nym
/// address then all entries with the matching announcer will be attempted to removed.
pub(crate) fn delete_nym_address(
    deps: DepsMut,
    info: MessageInfo,
    nym_address: NymAddress,
) -> Result<Response> {
    let mut response = Response::new();
    let services_to_delete = query::query_nym_address(deps.as_ref(), nym_address)?.services;

    for service_to_delete in services_to_delete {
        if info.sender == service_to_delete.announcer {
            state::remove(deps.storage, service_to_delete.service_id)?;
            let return_deposit_msg = return_deposit(&service_to_delete);
            response = response
                .add_message(return_deposit_msg)
                .add_event(new_delete_id_event(service_to_delete));
        }
    }
    Ok(response)
}

/// Update the deposit required to announce new services
pub(crate) fn update_deposit_required(
    deps: DepsMut,
    info: MessageInfo,
    deposit_required: Coin,
) -> Result<Response> {
    state::assert_admin(deps.as_ref(), &info.sender)?;

    let mut config = state::load_config(deps.storage)?;
    config.deposit_required = deposit_required.clone();
    state::save_config(deps.storage, &config)?;

    Ok(Response::new().add_event(new_update_deposit_required_event(deposit_required)))
}
