import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { MixNode } from './types'

export interface INetClient {
    getMixnodes(page: number, perPage: number): MixNode[];
}

export default class NetClient implements INetClient {

    private cosmos: SigningCosmWasmClient;

    constructor(cosmos: SigningCosmWasmClient) {
        this.cosmos = cosmos;
    }

    public getMixnodes(page: number, perPage: number): MixNode[] {
        return [];
    }

}
