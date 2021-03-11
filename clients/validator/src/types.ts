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
    version: string,
}