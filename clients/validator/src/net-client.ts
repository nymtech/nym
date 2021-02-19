import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-launchpad';
import { MixNode } from './types'

export default class NetClient {

    cosmos: SigningCosmWasmClient;

    constructor(cosmos: SigningCosmWasmClient) {
        this.cosmos = cosmos;
    }

    public getMixnodes(page: number, perPage: number): MixNode[] {
        return [];
    }

}
