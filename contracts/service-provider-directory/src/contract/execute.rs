
use cosmwasm_std::Coin;

use super::*;
use crate::state::{self, NymAddress, ServiceId, ServiceType};

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

    if will_deposit < deposit_required.amount {
        return Err(ContractError::InsufficientDeposit {
            funds: will_deposit,
            deposit_required,
        });
    }

    if will_deposit > deposit_required.amount {
        return Err(ContractError::TooLargeDeposit {
            funds: will_deposit,
            deposit_required,
        });
    }

    let will_deposit = Coin::new(will_deposit.u128(), denom);

    let new_service = Service {
        nym_address,
        service_type,
        owner,
        block_height: env.block.height,
        deposit: will_deposit,
    };
    let service_id = state::next_service_id_counter(deps.storage)?;
    SERVICES.save(deps.storage, service_id, &new_service)?;
    Ok(Response::new()
        //.add_message(deposit_msg)
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
    if !SERVICES.has(deps.storage, service_id) {
        return Err(ContractError::NotFound { service_id });
    }

    let service_to_delete = SERVICES.load(deps.storage, service_id)?;
    if info.sender != service_to_delete.owner {
        return Err(ContractError::Unauthorized {
            sender: info.sender,
        });
    }

    //let return_deposit_msg = BankMsg::Send {
    //to_address: service_to_delete.owner.to_string(),
    //amount: vec![service_to_delete.deposit],
    //};

    SERVICES.remove(deps.storage, service_id);
    Ok(Response::new()
        //.add_message(return_deposit_msg)
        .add_attribute("action", "delete")
        .add_attribute("service_id", service_id.to_string()))
}
