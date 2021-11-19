use crate::error::ContractError;
use crate::storage::{
    decrement_layer_count, gateways, gateways_owners, gateways_owners_read, gateways_read,
    increment_layer_count, mixnodes_owners_read, read_state_params,
};
use config::defaults::DENOM;
use cosmwasm_std::{attr, BankMsg, Coin, DepsMut, Env, MessageInfo, Response, Uint128};
use mixnet_contract::{Gateway, GatewayBond, Layer};

pub fn validate_gateway_bond(bond: &[Coin], minimum_bond: Uint128) -> Result<(), ContractError> {
    // check if anything was put as bond
    if bond.is_empty() {
        return Err(ContractError::NoBondFound);
    }

    if bond.len() > 1 {
        return Err(ContractError::MultipleDenoms);
    }

    // check that the denomination is correct
    if bond[0].denom != DENOM {
        return Err(ContractError::WrongDenom {});
    }

    // check that we have at least 100 coins in our bond
    if bond[0].amount < minimum_bond {
        return Err(ContractError::InsufficientGatewayBond {
            received: bond[0].amount.into(),
            minimum: minimum_bond.into(),
        });
    }

    Ok(())
}

pub(crate) fn try_add_gateway(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    gateway: Gateway,
) -> Result<Response, ContractError> {
    let sender_bytes = info.sender.as_bytes();

    // if the client has an active bonded mixnode, don't allow gateway bonding
    if mixnodes_owners_read(deps.storage)
        .may_load(sender_bytes)?
        .is_some()
    {
        return Err(ContractError::AlreadyOwnsMixnode);
    }

    let mut was_present = false;
    // if the client has an active gateway with a different identity, don't allow bonding
    if let Some(existing_node) = gateways_owners_read(deps.storage).may_load(sender_bytes)? {
        if existing_node != gateway.identity_key {
            return Err(ContractError::AlreadyOwnsGateway);
        }
        was_present = true
    }

    // check if somebody else has already bonded a gateway with this identity
    if let Some(existing_bond) =
        gateways_read(deps.storage).may_load(gateway.identity_key.as_bytes())?
    {
        if existing_bond.owner != info.sender {
            return Err(ContractError::DuplicateGateway {
                owner: existing_bond.owner,
            });
        }
    }

    let minimum_bond = read_state_params(deps.storage).minimum_gateway_bond;
    validate_gateway_bond(&info.funds, minimum_bond)?;

    let bond = GatewayBond::new(
        info.funds[0].clone(),
        info.sender.clone(),
        env.block.height,
        gateway,
    );

    let identity = bond.identity();
    gateways(deps.storage).save(identity.as_bytes(), &bond)?;
    gateways_owners(deps.storage).save(sender_bytes, identity)?;
    increment_layer_count(deps.storage, Layer::Gateway)?;

    let attributes = vec![attr("overwritten", was_present)];
    Ok(Response {
        submessages: Vec::new(),
        messages: Vec::new(),
        attributes,
        data: None,
    })
}

pub(crate) fn try_remove_gateway(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let sender_bytes = info.sender.as_str().as_bytes();

    // try to find the identity of the sender's node
    let gateway_identity = match gateways_owners_read(deps.storage).may_load(sender_bytes)? {
        Some(identity) => identity,
        None => return Err(ContractError::NoAssociatedGatewayBond { owner: info.sender }),
    };

    // get the bond, since we found associated identity, the node MUST exist
    let gateway_bond = gateways_read(deps.storage).load(gateway_identity.as_bytes())?;

    // send bonded funds back to the bond owner
    let messages = vec![BankMsg::Send {
        to_address: info.sender.as_str().to_owned(),
        amount: vec![gateway_bond.bond_amount()],
    }
    .into()];

    // remove the bond from the list of bonded gateways
    gateways(deps.storage).remove(gateway_identity.as_bytes());
    // remove the node ownership
    gateways_owners(deps.storage).remove(sender_bytes);
    // decrement layer count
    decrement_layer_count(deps.storage, Layer::Gateway)?;

    // log our actions
    let attributes = vec![
        attr("action", "unbond"),
        attr("address", info.sender),
        attr("gateway_bond", gateway_bond),
    ];

    Ok(Response {
        submessages: Vec::new(),
        messages,
        attributes,
        data: None,
    })
}
