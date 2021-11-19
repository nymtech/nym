use crate::error::ContractError;
use crate::storage::{
    decrement_layer_count, gateways_owners_read, increment_layer_count, mix_delegations_read,
    mixnodes, mixnodes_owners, mixnodes_owners_read, mixnodes_read, read_state_params,
};
use config::defaults::DENOM;
use cosmwasm_std::{attr, BankMsg, Coin, DepsMut, Env, MessageInfo, Response, Uint128};
use mixnet_contract::{MixNode, MixNodeBond};

pub fn validate_mixnode_bond(bond: &[Coin], minimum_bond: Uint128) -> Result<(), ContractError> {
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

    // check that we have at least MIXNODE_BOND coins in our bond
    if bond[0].amount < minimum_bond {
        return Err(ContractError::InsufficientMixNodeBond {
            received: bond[0].amount.into(),
            minimum: minimum_bond.into(),
        });
    }

    Ok(())
}

pub(crate) fn try_add_mixnode(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mix_node: MixNode,
) -> Result<Response, ContractError> {
    let sender_bytes = info.sender.as_bytes();

    // if the client has an active bonded gateway, don't allow mixnode bonding
    if gateways_owners_read(deps.storage)
        .may_load(sender_bytes)?
        .is_some()
    {
        return Err(ContractError::AlreadyOwnsGateway);
    }

    let mut was_present = false;
    // if the client has an active mixnode with a different identity, don't allow bonding
    if let Some(existing_node) = mixnodes_owners_read(deps.storage).may_load(sender_bytes)? {
        if existing_node != mix_node.identity_key {
            return Err(ContractError::AlreadyOwnsMixnode);
        }
        was_present = true
    }

    // check if somebody else has already bonded a mixnode with this identity
    if let Some(existing_bond) =
        mixnodes_read(deps.storage).may_load(mix_node.identity_key.as_bytes())?
    {
        if existing_bond.owner != info.sender {
            return Err(ContractError::DuplicateMixnode {
                owner: existing_bond.owner,
            });
        }
    }

    let minimum_bond = read_state_params(deps.storage).minimum_mixnode_bond;
    validate_mixnode_bond(&info.funds, minimum_bond)?;

    let layer_distribution = crate::queries::query_layer_distribution(deps.as_ref());
    let layer = layer_distribution.choose_with_fewest();

    let mut bond = MixNodeBond::new(
        info.funds[0].clone(),
        info.sender.clone(),
        layer,
        env.block.height,
        mix_node,
        None,
    );

    // this might potentially require more gas if a significant number of delegations was there
    let delegations_bucket = mix_delegations_read(deps.storage, &bond.mix_node.identity_key);
    let existing_delegation =
        crate::delegating_mixnodes::transactions::total_delegations(delegations_bucket)?;
    bond.total_delegation = existing_delegation;

    let identity = bond.identity();

    mixnodes(deps.storage).save(identity.as_bytes(), &bond)?;
    mixnodes_owners(deps.storage).save(sender_bytes, identity)?;
    increment_layer_count(deps.storage, bond.layer)?;

    let attributes = vec![attr("overwritten", was_present)];
    Ok(Response {
        submessages: Vec::new(),
        messages: Vec::new(),
        attributes,
        data: None,
    })
}

pub(crate) fn try_remove_mixnode(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let sender_bytes = info.sender.as_bytes();

    // try to find the identity of the sender's node
    let mix_identity = match mixnodes_owners_read(deps.storage).may_load(sender_bytes)? {
        Some(identity) => identity,
        None => return Err(ContractError::NoAssociatedMixNodeBond { owner: info.sender }),
    };

    // get the bond, since we found associated identity, the node MUST exist
    let mixnode_bond = mixnodes_read(deps.storage).load(mix_identity.as_bytes())?;

    // send bonded funds back to the bond owner
    let messages = vec![BankMsg::Send {
        to_address: info.sender.as_str().to_owned(),
        amount: vec![mixnode_bond.bond_amount()],
    }
    .into()];

    // remove the bond from the list of bonded mixnodes
    mixnodes(deps.storage).remove(mix_identity.as_bytes());
    // remove the node ownership
    mixnodes_owners(deps.storage).remove(sender_bytes);
    // decrement layer count
    decrement_layer_count(deps.storage, mixnode_bond.layer)?;

    // log our actions
    let attributes = vec![attr("action", "unbond"), attr("mixnode_bond", mixnode_bond)];

    Ok(Response {
        submessages: Vec::new(),
        messages,
        attributes,
        data: None,
    })
}
