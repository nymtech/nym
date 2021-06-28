import { Coin } from "@cosmjs/launchpad/";

export type MixNodeBond = { // TODO: change name to MixNodeBond
    owner: string,
    mix_node: MixNode,    // TODO: camelCase this later once everything else works

    amount: Coin[],
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
    amount: Coin[],
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