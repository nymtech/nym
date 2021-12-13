import {Coin} from "@cosmjs/stargate";


// TODO: ideally we'd have re-exported those using that fancy crate that builds ts types from rust

export type MixnetContractVersion = {
    build_timestamp: string,
    build_version: string,
    commit_sha: string,
    commit_timestamp: string,
    commit_branch: string,
    rustc_version: string,
}

export type PagedMixnodeResponse = {
    nodes: MixNodeBond[],
    per_page: number,
    start_next_after?: string,
}

export type PagedGatewayResponse = {
    nodes: GatewayBond[],
    per_page: number,
    start_next_after?: string,
}

export type MixOwnershipResponse = {
    address: string,
    mixnode?: MixNodeBond,
}

export type GatewayOwnershipResponse = {
    address: string,
    gateway?: GatewayBond,
}

export type ContractStateParams = {
    // ideally I'd want to define those as `number` rather than `string`, but
    // rust-side they are defined as Uint128 and that don't have
    // native javascript representations and therefore are interpreted as strings after deserialization
    minimum_mixnode_pledge: string
    minimum_gateway_pledge: string,
    mixnode_rewarded_set_size: number,
    mixnode_active_set_size: number,
}

export type RewardingIntervalResponse = {
    current_rewarding_interval_starting_block: number,
    current_rewarding_interval_nonce: number,
    rewarding_in_progress: boolean,
}

export type LayerDistribution = {
    gateways: number,
    layer1: number,
    layer2: number,
    layer3: number,
}

export type Delegation = {
    owner: string,
    node_identity: string,
    amount: Coin,
    block_height: number,
    proxy?: string
}

export type PagedMixDelegationsResponse = {
    delegations: Delegation[],
    start_next_after?: string
}

export type PagedDelegatorDelegationsResponse = {
    delegations: Delegation[],
    start_next_after?: string
}

export type PagedAllDelegationsResponse = {
    delegations: Delegation[],
    start_next_after?: [string, string],
}

export type RewardingResult = {
    operator_reward: string,
    total_delegator_reward: string,
}

export type NodeRewardParams = {
    period_reward_pool: string,
    k: string,
    reward_blockstamp: number,
    circulating_supply: string,
    uptime: string,
    sybil_resistance_percent: number,
}

export type DelegatorRewardParams = {
    node_reward_params: NodeRewardParams,
    sigma: number,
    profit_margin: number,
    node_profit: number,
}

export type PendingDelegatorRewarding = {
    running_results: RewardingResult,
    next_start: string,
    rewarding_params: DelegatorRewardParams,
}

export type RewardingStatus = { Complete: RewardingResult } | { PendingNextDelegatorPage: PendingDelegatorRewarding };

export type MixnodeRewardingStatusResponse = {
    status?: RewardingStatus
}

export enum Layer {
    Gateway,
    One,
    Two,
    Three,
}

export type MixNodeBond = {
    owner: string,
    mix_node: MixNode,
    layer: Layer,
    bond_amount: Coin,
    total_delegation: Coin,
}

export type MixNode = {
    host: string,
    mix_port: number,
    verloc_port: number,
    http_api_port: number,
    sphinx_key: string,
    identity_key: string,
    version: string,
}

export type GatewayBond = {
    owner: string
    gateway: Gateway,

    bond_amount: Coin,
    total_delegation: Coin,
}

export type Gateway = {
    host: string,
    mix_port: number,
    clients_port: number,
    location: string,
    sphinx_key: string,
    identity_key: string,
    version: string
}

export type SendRequest = {
    senderAddress: string,
    recipientAddress: string,
    transferAmount: readonly Coin[]
}