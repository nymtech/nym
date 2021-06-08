use crate::helpers::calculate_epoch_reward_rate;
use crate::msg::{HandleMsg, InitMsg, MigrateMsg, QueryMsg};
use crate::state::{State, StateParams};
use crate::storage::{config, layer_distribution};
use crate::{error::ContractError, queries, transactions};
use cosmwasm_std::{
    to_binary, Decimal, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse, MessageInfo,
    MigrateResponse, QueryResponse, Uint128,
};
use mixnet_contract::LayerDistribution;

pub const INITIAL_DEFAULT_EPOCH_LENGTH: u32 = 2;

/// Constant specifying minimum of coin required to bond a gateway
pub const INITIAL_GATEWAY_BOND: Uint128 = Uint128(100_000000);

/// Constant specifying minimum of coin required to bond a mixnode
pub const INITIAL_MIXNODE_BOND: Uint128 = Uint128(100_000000);

// percentage annual increase. Given starting value of x, we expect to have 1.1x at the end of the year
pub const INITIAL_MIXNODE_BOND_REWARD_RATE: u64 = 110;
pub const INITIAL_GATEWAY_BOND_REWARD_RATE: u64 = 110;

pub const INITIAL_MIXNODE_ACTIVE_SET_SIZE: u32 = 100;

const NETWORK_MONITOR_ADDRESS: &str = "hal1v9qauwdq5terag6uvfsdytcs2d0sdmfdq6e83g";

/// Constant specifying denomination of the coin used for bonding
pub const DENOM: &str = "uhal";

fn default_initial_state(owner: HumanAddr) -> State {
    let mixnode_bond_reward_rate = Decimal::percent(INITIAL_MIXNODE_BOND_REWARD_RATE);
    let gateway_bond_reward_rate = Decimal::percent(INITIAL_GATEWAY_BOND_REWARD_RATE);

    State {
        owner,
        network_monitor_address: NETWORK_MONITOR_ADDRESS.into(),
        params: StateParams {
            epoch_length: INITIAL_DEFAULT_EPOCH_LENGTH,
            minimum_mixnode_bond: INITIAL_MIXNODE_BOND,
            minimum_gateway_bond: INITIAL_GATEWAY_BOND,
            mixnode_bond_reward_rate,
            gateway_bond_reward_rate,
            mixnode_active_set_size: INITIAL_MIXNODE_ACTIVE_SET_SIZE,
        },
        mixnode_epoch_bond_reward: calculate_epoch_reward_rate(
            INITIAL_DEFAULT_EPOCH_LENGTH,
            mixnode_bond_reward_rate,
        ),
        gateway_epoch_bond_reward: calculate_epoch_reward_rate(
            INITIAL_DEFAULT_EPOCH_LENGTH,
            gateway_bond_reward_rate,
        ),
    }
}

/// Instantiate the contract.
///
/// `deps` contains Storage, API and Querier
/// `env` contains block, message and contract info
/// `msg` is the contract initialization message, sort of like a constructor call.
pub fn init(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    let state = default_initial_state(info.sender);

    config(deps.storage).save(&state)?;
    layer_distribution(deps.storage).save(&Default::default())?;
    Ok(InitResponse::default())
}

/// Handle an incoming message
pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::BondMixnode { mix_node } => transactions::try_add_mixnode(deps, info, mix_node),
        HandleMsg::UnbondMixnode {} => transactions::try_remove_mixnode(deps, info, env),
        HandleMsg::BondGateway { gateway } => transactions::try_add_gateway(deps, info, gateway),
        HandleMsg::UnbondGateway {} => transactions::try_remove_gateway(deps, info, env),
        HandleMsg::UpdateStateParams(params) => {
            transactions::try_update_state_params(deps, info, params)
        }
        HandleMsg::RewardMixnode { owner, uptime } => {
            transactions::try_reward_mixnode(deps, info, owner, uptime)
        }
        HandleMsg::RewardGateway { owner, uptime } => {
            transactions::try_reward_gateway(deps, info, owner, uptime)
        }
        HandleMsg::DelegateToMixnode { node_owner } => {
            transactions::try_delegate_to_mixnode(deps, info, node_owner)
        }
        HandleMsg::UndelegateFromMixnode { node_owner } => {
            transactions::try_remove_delegation_from_mixnode(deps, info, env, node_owner)
        }
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<QueryResponse, ContractError> {
    let query_res = match msg {
        QueryMsg::GetMixNodes { start_after, limit } => {
            to_binary(&queries::query_mixnodes_paged(deps, start_after, limit)?)
        }
        QueryMsg::GetGateways { limit, start_after } => {
            to_binary(&queries::query_gateways_paged(deps, start_after, limit)?)
        }
        QueryMsg::OwnsMixnode { address } => {
            to_binary(&queries::query_owns_mixnode(deps, address)?)
        }
        QueryMsg::OwnsGateway { address } => {
            to_binary(&queries::query_owns_gateway(deps, address)?)
        }
        QueryMsg::StateParams {} => to_binary(&queries::query_state_params(deps)),
        QueryMsg::GetMixDelegations {
            node_owner,
            start_after,
            limit,
        } => to_binary(&queries::query_mixnode_delegations_paged(
            deps,
            node_owner,
            start_after,
            limit,
        )?),
        QueryMsg::GetMixDelegation {
            node_owner,
            address,
        } => to_binary(&queries::query_mixnode_delegation(
            deps, node_owner, address,
        )?),
        QueryMsg::LayerDistribution {} => to_binary(&queries::query_layer_distribution(deps)),
    };

    Ok(query_res?)
}

pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: MigrateMsg,
) -> Result<MigrateResponse, ContractError> {
    // load all mixnodes and gateways and build up layer distribution
    let mut layers: LayerDistribution = Default::default();

    // go through mixnodes...
    let mut start_after = None;
    loop {
        let response = queries::query_mixnodes_paged(deps.as_ref(), start_after, None)?;
        start_after = response.start_next_after;
        if start_after.is_none() {
            break;
        }
        for node in response.nodes.into_iter() {
            match node.mix_node.layer {
                n if n == 1 => layers.layer1 += 1,
                n if n == 2 => layers.layer2 += 1,
                n if n == 3 => layers.layer3 += 1,
                _ => layers.invalid += 1,
            }
        }
    }

    // go through gateways...
    loop {
        let response = queries::query_gateways_paged(deps.as_ref(), start_after, None)?;
        start_after = response.start_next_after;
        if start_after.is_none() {
            break;
        }
        layers.gateways += response.nodes.len() as u64;
    }

    layer_distribution(deps.storage).save(&layers)?;

    Ok(Default::default())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::storage::{gateways, layer_distribution_read, mixnodes};
    use crate::support::tests::helpers;
    use crate::support::tests::helpers::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};
    use mixnet_contract::{Gateway, GatewayBond, MixNode, MixNodeBond, PagedResponse};

    #[test]
    fn initialize_contract() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let msg = InitMsg {};
        let info = mock_info("creator", &[]);

        let res = init(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // mix_node_bonds should be empty after initialization
        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::GetMixNodes {
                start_after: None,
                limit: Option::from(2),
            },
        )
        .unwrap();
        let page: PagedResponse = from_binary(&res).unwrap();
        assert_eq!(0, page.nodes.len()); // there are no mixnodes in the list when it's just been initialized

        // Contract balance should match what we initialized it as
        assert_eq!(
            coins(0, DENOM),
            query_contract_balance(env.contract.address, deps)
        );
    }

    // TODO: this test will have to be removed once the migration happens and we are working on yet another
    // version of the contract
    #[test]
    fn migration_to_layer_distribution() {
        let layer_ones = 42;
        let layer_twos = 123;
        let layer_threes = 111;
        let invalid = 30;
        let gateways_count = 24;

        // bond some nodes
        let mut deps = helpers::init_contract();
        let env = mock_env();

        for i in 0..layer_ones {
            let owner = HumanAddr::from(format!("owner{}{}", 1, i));
            mixnodes(&mut deps.storage)
                .save(
                    owner.clone().as_bytes(),
                    &MixNodeBond {
                        amount: coins(1000, "uhal"),
                        owner,
                        mix_node: MixNode {
                            host: "1.1.1.1:1111".to_string(),
                            layer: 1,
                            location: "aaaa".to_string(),
                            sphinx_key: "bbbb".to_string(),
                            identity_key: format!("identity{}{}", 1, i),
                            version: "0.10.1".to_string(),
                        },
                    },
                )
                .unwrap();
        }

        for i in 0..layer_twos {
            let owner = HumanAddr::from(format!("owner{}{}", 2, i));
            mixnodes(&mut deps.storage)
                .save(
                    owner.clone().as_bytes(),
                    &MixNodeBond {
                        amount: coins(1000, "uhal"),
                        owner,
                        mix_node: MixNode {
                            host: "1.1.1.1:1111".to_string(),
                            layer: 2,
                            location: "aaaa".to_string(),
                            sphinx_key: "bbbb".to_string(),
                            identity_key: format!("identity{}{}", 2, i),
                            version: "0.10.1".to_string(),
                        },
                    },
                )
                .unwrap();
        }

        for i in 0..layer_threes {
            let owner = HumanAddr::from(format!("owner{}{}", 3, i));
            mixnodes(&mut deps.storage)
                .save(
                    owner.clone().as_bytes(),
                    &MixNodeBond {
                        amount: coins(1000, "uhal"),
                        owner,
                        mix_node: MixNode {
                            host: "1.1.1.1:1111".to_string(),
                            layer: 3,
                            location: "aaaa".to_string(),
                            sphinx_key: "bbbb".to_string(),
                            identity_key: format!("identity{}{}", 3, i),
                            version: "0.10.1".to_string(),
                        },
                    },
                )
                .unwrap();
        }

        for i in 0..invalid {
            let owner = HumanAddr::from(format!("owner{}{}", 42, i));
            mixnodes(&mut deps.storage)
                .save(
                    owner.clone().as_bytes(),
                    &MixNodeBond {
                        amount: coins(1000, "uhal"),
                        owner,
                        mix_node: MixNode {
                            host: "1.1.1.1:1111".to_string(),
                            layer: 42,
                            location: "aaaa".to_string(),
                            sphinx_key: "bbbb".to_string(),
                            identity_key: format!("identity{}{}", 42, i),
                            version: "0.10.1".to_string(),
                        },
                    },
                )
                .unwrap();
        }

        for i in 0..gateways_count {
            let owner = HumanAddr::from(format!("owner{}{}", "gateway", i));
            gateways(&mut deps.storage)
                .save(
                    owner.clone().as_bytes(),
                    &GatewayBond {
                        amount: coins(1000, "uhal"),
                        owner,
                        gateway: Gateway {
                            mix_host: "1.1.1.1:1111".to_string(),
                            clients_host: "ws://1.1.1.1:1112".to_string(),
                            location: "aaaa".to_string(),
                            sphinx_key: "bbbb".to_string(),
                            identity_key: format!("identity{}{}", "gateway", i),
                            version: "0.10.1".to_string(),
                        },
                    },
                )
                .unwrap();
        }

        let migrate_res = migrate(deps.as_mut(), env, mock_info("creator", &[]), MigrateMsg {});
        assert!(migrate_res.is_ok());

        let layers = layer_distribution_read(&deps.storage).load().unwrap();
        let expected = LayerDistribution {
            gateways: gateways_count,
            layer1: layer_ones,
            layer2: layer_twos,
            layer3: layer_threes,
            invalid,
        };
        assert_eq!(expected, layers);
    }
}
