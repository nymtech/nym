import MixnodesCache from "./caches/mixnodes";
import { Coin } from "@cosmjs/launchpad/";

export interface MixNodeBond { // TODO: change name to MixNodeBond
    owner: string,
    mix_node: MixNode,    // TODO: camelCase this later once everything else works

    amount: Coin[],
}

export interface MixNode {
    host: string,
    layer: number,
    location: string,
    sphinx_key: string, // TODO: camelCase this later once everything else works
    version: string,
}