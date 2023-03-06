import { Coin } from 'cosmjs-types/cosmos/base/v1beta1/coin';
import expect from 'expect';

export const amountDemon = {
    denom: expect.any(String),
    amount: expect.any(String)
}

export const delegation = {
    owner: expect.any(String),
    mix_id: expect.any(Number),
    cumulative_reward_ratio: expect.any(String),
    amount: amountDemon,
    height: expect.any(Number || BigInt),
    proxy: expect.any(String || null)
}

export const detailedDelegation = {
    delegation: delegation,
    mixnode_still_bonded: expect.any(Boolean)
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

export const stateParams = {
    minimum_gateway_pledge: amountDemon,
    minimum_mixnode_pledge: expect.any(String),
    mixnode_rewarded_set_size: expect.any(Number),
    mixnode_active_set_size: expect.any(Number)
}

export const contract = {
    owner: expect.any(Number),
    rewarding_validator_address: expect.any(Number),
    vesting_contract_address: expect.any(Number),
    rewarding_denom: expect.any(String),
    params: stateParams
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

export const VestAccounts = [{
    account_id: expect.any(String),
    owner: expect.any(String)
}]

export const VestAccountCoin = [{
    account_id: expect.any(String),
    owner: expect.any(String),
    still_vesting: Coin
}]

export const vestingAccountsPaged = {
    accounts: VestAccounts,
    start_next_after: expect.any(String)
}

export const VestingCoinAccounts = {
    accounts: VestAccountCoin,
    start_next_after: expect.any(String)
}

export const OriginalVestingDetails = {
    amount: Coin,
    number_of_periods: expect.any(Number),
    period_duration: expect.any(Number)
}

export const PledgeCap = {
    percent: expect.any(String) || null,
};

export const Periods = [{
    period_seconds: expect.any(Number),
    start_time: expect.any(Number),
}]

export const VestingAccountDetails = {
    owner_address: expect.any(String),
    staking_address: expect.any(String) || null,
    start_time: expect.any(String),
    periods: Periods,
    coin: Coin,
    storage_key: expect.any(Number),
    pledge_cap: PledgeCap
}

export const Node = {
    amount: Coin,
    block_time: expect.any(String)
}

export type VestingPeriod = 'Before' | { In: number } | 'After';

export const DelegationTimestamps = [
    expect.any(Number)
]

export const DelegatorTimes = {
    owner: expect.any(String),
    account_id: expect.any(Number),
    mix_id: expect.any(Number),
    delegation_timestamps: DelegationTimestamps
}

export const DelegationBlock = [{
    account_id: expect.any(Number),
    amount: expect.any(String),
    block_timestamp: expect.any(Number),
    mix_id: expect.any(Number)
}]

export const Delegations = {
    delegations: DelegationBlock,
    start_next_after: expect.any(String) || null
}
