import expect from 'expect';

export const amountDemon = {
    amount: expect.any(String),
    denom: expect.any(String)
}

export const delegation = {
    owner: expect.any(String),
    mix_id: expect.any(Number),
    cumulative_reward_ratio: expect.any(String),
    amount: amountDemon,
    height: expect.any(Number || BigInt),
    proxy: expect.any(String || null)
}

export const gateway = {
    pledge_amount: amountDemon,
    owner: expect.any(String),
    block_height: expect.any(Number || BigInt),
    gateway: {
        host: expect.any(String),
        mix_port: expect.any(Number),
        clients_port: expect.any(Number),
        location: expect.any(String),
        sphinx_key: expect.any(String),
        identity_key: expect.any(String),
        version: expect.any(String),
    },
    proxy: expect.any(String || null)
}

export const pagedGateway = {
    nodes: gateway,
    per_page: expect.any(Number),
    start_next_after: expect.any(Number)
}

export const ownGateway = {
    address: expect.any(String),
    gateway: gateway
}

export const rewardingdetails = {
    cost_params: {
        profit_margin_percent: expect.any(String),
        interval_operating_cost: {
            denom: expect.any(String),
            amount: expect.any(String)
        }
    },
    operator: expect.any(String),
    delegates: expect.any(String),
    total_unit_reward: expect.any(String),
    unit_delegation: expect.any(String),
    last_rewarded_epoch: expect.any(Number),
    unique_delegations: expect.any(Number)
}

export const mix_node = {
    host: expect.any(String),
    mix_port: expect.any(Number),
    verloc_port: expect.any(Number),
    http_api_port: expect.any(Number),
    sphinx_key: expect.any(String),
    identity_key: expect.any(String),
    version: expect.any(String)
}

export const mixnodebond = {
    mix_id: expect.any(Number),
    owner: expect.any(String),
    original_pledge: {
        denom: expect.any(String),
        amount: expect.any(String)
    },
    layer: expect.any(String),
    mix_node: mix_node,
    proxy: expect.any(String) || null,
    bonding_height: expect.any(Number || BigInt),
    is_unbonding: expect.any(Boolean)
}

export const mixnode = {
    bond_information: mixnodebond,
    rewarding_details: rewardingdetails
}

export const ownedNode = {
    address: expect.any(String),
    mixnode_details: {
        bond_information: mixnodebond,
        rewarding_details: rewardingdetails
    }
}

export const saturation = {
    mix_id: expect.any(Number),
    current_saturation: expect.any(String),
    uncapped_saturation: expect.any(String)
}

export const contractVersion = {
    build_timestamp: expect.any(String),
    build_version: expect.any(String),
    commit_sha: expect.any(String),
    commit_timestamp: expect.any(String),
    commit_branch: expect.any(String),
    rustc_version: expect.any(String)
};

export const contract = {
    minimum_gateway_pledge: amountDemon,
    minimum_mixnode_pledge: expect.any(String),
    mixnode_rewarded_set_size: expect.any(Number),
    mixnode_active_set_size: expect.any(Number)
}

export const rewardingnode = {
    mix_id: expect.any(Number),
    rewarding_details: rewardingdetails
}

export const unbondednode = {
    mix_id: expect.any(Number),
    unbonded_info: {
        identity_key: expect.any(String),
        owner: expect.any(String),
        proxy: expect.any(String) || null,
        unbonding_height: expect.any(Number)
    }
}

export const allunbondednodes = [
    expect.any(Number), {
        identity_key: expect.any(String),
        owner: expect.any(String),
        proxy: expect.any(String) || null,
        unbonding_height: expect.any(Number)
    }
]

export const layerDistribution = {
    layer1: expect.any(Number),
    layer2: expect.any(Number),
    layer3: expect.any(Number)
}


export const intervalRewardParams = {
    reward_pool: expect.any(Number),
    staking_supply: expect.any(Number),
    staking_supply_scale_factor: expect.any(Number),
    epoch_reward_budget: expect.any(Number),
    stake_saturation_point: expect.any(Number),
    sybil_resistance: expect.any(Number),
    active_set_work_factor: expect.any(Number),
    interval_pool_emission: expect.any(Number)
}

export const rewardingParams = {
    interval: intervalRewardParams,
    rewarded_set_size: expect.any(Number),
    active_set_size: expect.any(Number)
}
