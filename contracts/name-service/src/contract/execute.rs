use crate::{
    constants::{MAX_NUMBER_OF_NAMES_FOR_ADDRESS, MAX_NUMBER_OF_NAMES_PER_OWNER},
    state, NameServiceError, Result,
};
use cosmwasm_std::{Addr, BankMsg, Coin, Deps, DepsMut, Env, MessageInfo, Response, Uint128};
use nym_contracts_common::{
    signing::{MessageSignature, Verifier},
    IdentityKey,
};
use nym_name_service_common::{
    events::{
        new_delete_id_event, new_delete_name_event, new_register_event,
        new_update_deposit_required_event,
    },
    signing_types::construct_name_register_sign_payload,
    Address, NameDetails, NameId, NymName, RegisteredName,
};

use super::query;

fn ensure_correct_deposit(will_deposit: Uint128, deposit_required: Uint128) -> Result<()> {
    match will_deposit.cmp(&deposit_required) {
        std::cmp::Ordering::Less => Err(NameServiceError::InsufficientDeposit {
            funds: will_deposit,
            deposit_required,
        }),
        std::cmp::Ordering::Equal => Ok(()),
        std::cmp::Ordering::Greater => Err(NameServiceError::TooLargeDeposit {
            funds: will_deposit,
            deposit_required,
        }),
    }
}

fn ensure_max_names_per_owner(deps: Deps, owner: Addr) -> Result<()> {
    let current_entries = query::query_owner(deps, owner.to_string())?;
    if current_entries.names.len() < MAX_NUMBER_OF_NAMES_PER_OWNER as usize {
        Ok(())
    } else {
        Err(NameServiceError::ReachedMaxNamesForOwner {
            max_names: MAX_NUMBER_OF_NAMES_PER_OWNER,
            owner,
        })
    }
}

fn ensure_max_names_per_address(deps: Deps, address: Address) -> Result<()> {
    let current_entries = query::query_address(deps, address.clone())?;
    if current_entries.names.len() < MAX_NUMBER_OF_NAMES_FOR_ADDRESS as usize {
        Ok(())
    } else {
        Err(NameServiceError::ReachedMaxNamesForAddress {
            max_names: MAX_NUMBER_OF_NAMES_FOR_ADDRESS,
            address,
        })
    }
}

fn ensure_name_exists(deps: Deps, name_id: NameId) -> Result<()> {
    if state::names::has_name_id(deps.storage, name_id) {
        Ok(())
    } else {
        Err(NameServiceError::NotFound { name_id })
    }
}

fn ensure_name_not_exists(deps: Deps, name: &NymName) -> Result<()> {
    if state::names::has_name(deps.storage, name) {
        Err(NameServiceError::NameAlreadyRegistered { name: name.clone() })
    } else {
        Ok(())
    }
}

fn ensure_identity_key_is_part_of_nym_address(address: &Address, identity_key: &str) -> Result<()> {
    if address.client_id() == identity_key {
        Ok(())
    } else {
        Err(NameServiceError::IdentityKeyMismatch {
            address: address.clone(),
            identity_key: identity_key.to_string(),
        })
    }
}

fn ensure_sender_authorized(info: MessageInfo, names: &RegisteredName) -> Result<()> {
    if info.sender == names.owner {
        Ok(())
    } else {
        Err(NameServiceError::Unauthorized {
            sender: info.sender,
        })
    }
}

fn return_deposit(name_to_delete: &RegisteredName) -> BankMsg {
    BankMsg::Send {
        to_address: name_to_delete.owner.to_string(),
        amount: vec![name_to_delete.deposit.clone()],
    }
}

fn verify_register_signature(
    deps: Deps<'_>,
    sender: Addr,
    deposit: Coin,
    name: NameDetails,
    signature: MessageSignature,
) -> Result<()> {
    // recover the public key
    let public_key = decode_ed25519_identity_key(&name.identity_key)?;

    // reconstruct the payload
    let nonce = state::get_signing_nonce(deps.storage, sender.clone())?;

    let msg = construct_name_register_sign_payload(nonce, sender, deposit, name);

    if deps.api.verify_message(msg, signature, &public_key)? {
        Ok(())
    } else {
        Err(NameServiceError::InvalidEd25519Signature)
    }
}

fn decode_ed25519_identity_key(encoded: &IdentityKey) -> Result<[u8; 32]> {
    let mut public_key = [0u8; 32];
    let used = bs58::decode(encoded)
        .into(&mut public_key)
        .map_err(|err| NameServiceError::MalformedEd25519IdentityKey(err.to_string()))?;

    if used != 32 {
        return Err(NameServiceError::MalformedEd25519IdentityKey(
            "Too few bytes provided for the public key".into(),
        ));
    }

    Ok(public_key)
}

/// Register a new name. It will be assigned a new name id.
pub fn register(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    name: NameDetails,
    owner_signature: MessageSignature,
) -> Result<Response> {
    ensure_identity_key_is_part_of_nym_address(&name.address, &name.identity_key)?;
    ensure_name_not_exists(deps.as_ref(), &name.name)?;
    ensure_max_names_per_owner(deps.as_ref(), info.sender.clone())?;
    ensure_max_names_per_address(deps.as_ref(), name.address.clone())?;

    let deposit_required = state::deposit_required(deps.storage)?;
    let denom = deposit_required.denom.clone();
    let will_deposit = cw_utils::must_pay(&info, &denom)
        .map_err(|err| NameServiceError::DepositRequired { source: err })?;
    ensure_correct_deposit(will_deposit, deposit_required.amount)?;

    let deposit = Coin::new(will_deposit.u128(), denom);

    verify_register_signature(
        deps.as_ref(),
        info.sender.clone(),
        deposit.clone(),
        name.clone(),
        owner_signature,
    )?;

    state::increment_signing_nonce(deps.storage, info.sender.clone())?;

    let id = state::next_name_id_counter(deps.storage)?;
    let new_name = RegisteredName {
        id,
        name,
        owner: info.sender,
        block_height: env.block.height,
        deposit,
    };
    state::names::save(deps.storage, &new_name)?;

    Ok(Response::new().add_event(new_register_event(new_name)))
}

/// Delete an exsisting name.
pub fn delete_id(deps: DepsMut, info: MessageInfo, name_id: NameId) -> Result<Response> {
    ensure_name_exists(deps.as_ref(), name_id)?;
    let name_to_delete = state::names::load_id(deps.storage, name_id)?;
    ensure_sender_authorized(info, &name_to_delete)?;

    state::names::remove_id(deps.storage, name_id)?;
    let return_deposit_msg = return_deposit(&name_to_delete);

    Ok(Response::new()
        .add_message(return_deposit_msg)
        .add_event(new_delete_id_event(name_to_delete)))
}

/// Delete an existing name by name.
pub(crate) fn delete_name(deps: DepsMut, info: MessageInfo, name: NymName) -> Result<Response> {
    let name_to_delete = query::query_name(deps.as_ref(), name)?;
    ensure_sender_authorized(info, &name_to_delete)?;

    state::names::remove_id(deps.storage, name_to_delete.id)?;
    let return_deposit_msg = return_deposit(&name_to_delete);

    Ok(Response::new()
        .add_message(return_deposit_msg)
        .add_event(new_delete_name_event(name_to_delete)))
}

/// Update the deposit required to register new names
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
