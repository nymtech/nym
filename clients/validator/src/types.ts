import MixnodesCache from "./caches/mixnodes";
import { Coin } from "@cosmjs/launchpad/";

export interface MixNode { // TODO: change name to MixNodeBond
    owner: string,
    mix_node: { // TODO: camelCase this later once everything else works
        host: string,
        layer: number,
        location: string,
        sphinx_key: string, // TODO: camelCase this later once everything else works
        version: string,
    }
    amount: Coin[],
}
