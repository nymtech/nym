use cosmwasm_std::{Addr, BankMsg, Coin, DepsMut, Env, MessageInfo, Response, Uint128};

use crate::{
    error::ContractError,
    state::{self, NymAddress, Service, ServiceId, ServiceType},
};

fn ensure_correct_deposit(
    will_deposit: Uint128,
    deposit_required: Coin,
) -> Result<(), ContractError> {
    match will_deposit.cmp(&deposit_required.amount) {
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

/// Announce a new service. It will be assigned a new service provider id.
pub fn announce(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    nym_address: NymAddress,
    service_type: ServiceType,
    owner: Addr,
) -> Result<Response, ContractError> {
    let deposit_required = state::deposit_required(deps.storage)?;
    let denom = deposit_required.denom.clone();
    let will_deposit = cw_utils::must_pay(&info, &denom)
        .map_err(|err| ContractError::DepositRequired { source: err })?;
    ensure_correct_deposit(will_deposit, deposit_required)?;

    let new_service = Service {
        nym_address,
        service_type,
        owner,
        block_height: env.block.height,
        deposit: Coin::new(will_deposit.u128(), denom),
    };
    let service_id = state::next_service_id_counter(deps.storage)?;
    state::save_service(deps.storage, service_id, new_service)?;
    Ok(Response::new()
        .add_attribute("action", "announce")
        .add_attribute("service_id", service_id.to_string())
        .add_attribute("service_type", service_type.to_string()))
}

/// Delete an exsisting service.
pub fn delete(
    deps: DepsMut,
    info: MessageInfo,
    service_id: ServiceId,
) -> Result<Response, ContractError> {
    if !state::has_service(deps.storage, service_id) {
        return Err(ContractError::NotFound { service_id });
    }

    let service_to_delete = state::load_service(deps.storage, service_id)?;

    if info.sender != service_to_delete.owner {
        return Err(ContractError::Unauthorized {
            sender: info.sender,
        });
    }

    // TODO: should this be reduced to take transaction costs into account? So that the contract
    // doesn't run out of funds.
    let return_deposit_msg = BankMsg::Send {
        to_address: service_to_delete.owner.to_string(),
        amount: vec![service_to_delete.deposit],
    };

    state::remove_service(deps.storage, service_id);
    Ok(Response::new()
        .add_message(return_deposit_msg)
        .add_attribute("action", "delete")
        .add_attribute("service_id", service_id.to_string()))
}
