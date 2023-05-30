use crate::{
    constants::{MAX_NUMBER_OF_NAMES_FOR_ADDRESS, MAX_NUMBER_OF_NAMES_PER_OWNER},
    error::{NameServiceError, Result},
    state,
};
use cosmwasm_std::{Addr, BankMsg, Coin, Deps, DepsMut, Env, MessageInfo, Response, Uint128};
use nym_name_service_common::{
    events::{
        new_delete_id_event, new_delete_name_event, new_register_event,
        new_update_deposit_required_event,
    },
    Address, NameId, NymName, RegisteredName,
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
        println!("name already exists");
        Err(NameServiceError::NameAlreadyRegistered { name: name.clone() })
    } else {
        Ok(())
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

/// Register a new name. It will be assigned a new name id.
pub fn register(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    name: NymName,
    address: Address,
) -> Result<Response> {
    ensure_name_not_exists(deps.as_ref(), &name)?;
    ensure_max_names_per_owner(deps.as_ref(), info.sender.clone())?;
    ensure_max_names_per_address(deps.as_ref(), address.clone())?;

    let deposit_required = state::deposit_required(deps.storage)?;
    let denom = deposit_required.denom.clone();
    let will_deposit = cw_utils::must_pay(&info, &denom)
        .map_err(|err| NameServiceError::DepositRequired { source: err })?;
    ensure_correct_deposit(will_deposit, deposit_required.amount)?;

    let new_name = RegisteredName {
        address,
        name,
        owner: info.sender,
        block_height: env.block.height,
        deposit: Coin::new(will_deposit.u128(), denom),
    };
    let name_id = state::names::save(deps.storage, &new_name)?;

    Ok(Response::new().add_event(new_register_event(name_id, new_name)))
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
        .add_event(new_delete_id_event(name_id, name_to_delete)))
}

/// Delete an existing name by name.
pub(crate) fn delete_name(deps: DepsMut, info: MessageInfo, name: NymName) -> Result<Response> {
    let name_to_delete = query::query_name(deps.as_ref(), name)?;
    ensure_sender_authorized(info, &name_to_delete.name)?;

    state::names::remove_id(deps.storage, name_to_delete.name_id)?;
    let return_deposit_msg = return_deposit(&name_to_delete.name);

    Ok(Response::new()
        .add_message(return_deposit_msg)
        .add_event(new_delete_name_event(
            name_to_delete.name_id,
            name_to_delete.name,
        )))
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
