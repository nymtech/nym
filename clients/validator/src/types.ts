import { Coin } from "@cosmjs/launchpad/";

export type MixNodeBond = { // TODO: change name to MixNodeBond
    owner: string,
    mix_node: MixNode,    // TODO: camelCase this later once everything else works

    amount: Coin[],
}

export type MixNode = {
    host: string,
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
    mix_host: string,
    clients_host: string,
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