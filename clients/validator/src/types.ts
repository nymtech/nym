import { Coin } from "@cosmjs/stargate";


/// One page of a possible multi-page set of mixnodes. The paging interface is quite
/// inconvenient, as we don't have the two pieces of information we need to know
/// in order to do paging nicely (namely `currentPage` and `totalPages` parameters).
///
/// Instead, we have only `start_next_page_after`, i.e. the key of the last record
/// on this page. In order to get the *next* page, CosmWasm looks at that value,
/// finds the next record, and builds the next page starting there. This happens
/// **in series** rather than **in parallel** (!).
///
/// So we have some consistency problems:
///
/// * we can't make requests at a given block height, so the result set
///    which we assemble over time may change while requests are being made.
/// * at some point we will make a request for a `start_next_page_after` key
///   which has just been deleted from the database.
///
/// TODO: more robust error handling on the "deleted key" case.
export type PagedMixnodeResponse = {
    nodes: MixNodeBond[],
    per_page: number, // TODO: camelCase
    start_next_after: string, // TODO: camelCase
}

// a temporary way of achieving the same paging behaviour for the gateways
// the same points made for `PagedResponse` stand here.
export type PagedGatewayResponse = {
    nodes: GatewayBond[],
    per_page: number, // TODO: camelCase
    start_next_after: string, // TODO: camelCase
}

export type MixOwnershipResponse = {
    address: string,
    has_node: boolean,
}

export type GatewayOwnershipResponse = {
    address: string,
    has_gateway: boolean,
}

export type ContractSettingsParams = {
    epoch_length: number,
    // ideally I'd want to define those as `number` rather than `string`, but
    // rust-side they are defined as Uint128 and Decimal that don't have
    // native javascript representations and therefore are interpreted as strings after deserialization
    minimum_mixnode_bond: string,
    minimum_gateway_bond: string,
    mixnode_bond_reward_rate: string,
    gateway_bond_reward_rate: string,
    mixnode_delegation_reward_rate: string,
    gateway_delegation_reward_rate: string,
    mixnode_active_set_size: number,
    gateway_active_set_size: number,
}

export type Delegation = {
    owner: string,
    amount: Coin,
}

export type PagedMixDelegationsResponse = {
    node_owner: string,
    delegations: Delegation[],
    start_next_after: string
}

export type PagedGatewayDelegationsResponse = {
    node_owner: string,
    delegations: Delegation[],
    start_next_after: string
}


export enum Layer {
    Gateway,
    One,
    Two,
    Three,
}

export type MixNodeBond = { // TODO: change name to MixNodeBond
    owner: string,
    mix_node: MixNode,    // TODO: camelCase this later once everything else works
    layer: Layer,
    bond_amount: Coin,
    total_delegation: Coin,
}

export type MixNode = {
    host: string,
    mix_port: number,
    verloc_port: number,
    http_api_port: number,
    sphinx_key: string, // TODO: camelCase this later once everything else works
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