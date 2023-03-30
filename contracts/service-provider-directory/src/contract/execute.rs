use crate::{
    constants::{MAX_NUMBER_OF_ALIASES_FOR_NYM_ADDRESS, MAX_NUMBER_OF_PROVIDERS_PER_OWNER},
    error::{ContractError, Result},
    state,
};
use cosmwasm_std::{Addr, BankMsg, Coin, Deps, DepsMut, Env, MessageInfo, Response, Uint128};
use nym_service_provider_directory_common::{NymAddress, Service, ServiceId, ServiceType};

use super::query;

fn ensure_correct_deposit(will_deposit: Uint128, deposit_required: Uint128) -> Result<()> {
    match will_deposit.cmp(&deposit_required) {
        std::cmp::Ordering::Less => Err(ContractError::InsufficientDeposit {
            funds: will_deposit,
            deposit_required,
        }),
        std::cmp::Ordering::Equal => Ok(()),
        std::cmp::Ordering::Greater => Err(ContractError::TooLargeDeposit {
            funds: will_deposit,
            deposit_required,
        }),
    }
}

fn ensure_max_services_per_owner(deps: Deps, owner: Addr) -> Result<()> {
    let current_entries = query::query_owner(deps, owner.clone())?;
    if current_entries.services.len() < MAX_NUMBER_OF_PROVIDERS_PER_OWNER as usize {
        Ok(())
    } else {
        Err(ContractError::ReachedMaxProvidersForAdmin {
            max_providers: MAX_NUMBER_OF_PROVIDERS_PER_OWNER,
            owner,
        })
    }
}

fn ensure_max_aliases_per_nym_address(deps: Deps, nym_address: NymAddress) -> Result<()> {
    let current_entries = query::query_nym_address(deps, nym_address.clone())?;
    if current_entries.services.len() < MAX_NUMBER_OF_ALIASES_FOR_NYM_ADDRESS as usize {
        Ok(())
    } else {
        Err(ContractError::ReachedMaxAliasesForNymAddress {
            max_aliases: MAX_NUMBER_OF_ALIASES_FOR_NYM_ADDRESS,
            nym_address,
        })
    }
}

fn ensure_service_exists(deps: Deps, service_id: ServiceId) -> Result<()> {
    if state::services().has(deps.storage, service_id) {
        Ok(())
    } else {
        Err(ContractError::NotFound { service_id })
    }
}

fn ensure_sender_authorized(info: MessageInfo, service: &Service) -> Result<()> {
    if info.sender == service.owner {
        Ok(())
    } else {
        Err(ContractError::Unauthorized {
            sender: info.sender,
        })
    }
}

/// Announce a new service. It will be assigned a new service provider id.
pub fn announce(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    nym_address: NymAddress,
    service_type: ServiceType,
) -> Result<Response> {
    ensure_max_services_per_owner(deps.as_ref(), info.sender.clone())?;
    ensure_max_aliases_per_nym_address(deps.as_ref(), nym_address.clone())?;

    let deposit_required = state::deposit_required(deps.storage)?;
    let denom = deposit_required.denom.clone();
    let will_deposit = cw_utils::must_pay(&info, &denom)
        .map_err(|err| ContractError::DepositRequired { source: err })?;
    ensure_correct_deposit(will_deposit, deposit_required.amount)?;

    let new_service = Service {
        nym_address,
        service_type,
        owner: info.sender,
        block_height: env.block.height,
        deposit: Coin::new(will_deposit.u128(), denom),
    };
    let service_id = state::next_service_id_counter(deps.storage)?;
    state::services().save(deps.storage, service_id, &new_service)?;
    Ok(Response::new()
        .add_attribute("action", "announce")
        .add_attribute("service_id", service_id.to_string())
        .add_attribute("service_type", service_type.to_string()))
}

/// Delete an exsisting service.
pub fn delete(deps: DepsMut, info: MessageInfo, service_id: ServiceId) -> Result<Response> {
    ensure_service_exists(deps.as_ref(), service_id)?;
    let service_to_delete = state::services().load(deps.storage, service_id)?;
    ensure_sender_authorized(info, &service_to_delete)?;

    // TODO: should this be reduced to take transaction costs into account? So that the contract
    // doesn't run out of funds.
    let return_deposit_msg = BankMsg::Send {
        to_address: service_to_delete.owner.to_string(),
        amount: vec![service_to_delete.deposit],
    };

    state::services().remove(deps.storage, service_id)?;
    Ok(Response::new()
        .add_message(return_deposit_msg)
        .add_attribute("action", "delete")
        .add_attribute("service_id", service_id.to_string()))
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

    Ok(Response::new()
        .add_attribute("action", "update_deposit_required")
        .add_attribute("deposit_required", deposit_required.to_string()))
}
