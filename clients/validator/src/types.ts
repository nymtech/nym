import { Coin } from "@cosmjs/launchpad/";

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
    layer: number,
    location: string,
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