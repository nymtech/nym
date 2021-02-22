import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { MixNode, MixNodesResponse } from './types'

export interface INetClient {
    getMixnodes(page: number, perPage: number): Promise<MixNodesResponse>;
}

export default class NetClient implements INetClient {

    private cosmos: SigningCosmWasmClient;

    constructor(cosmos: SigningCosmWasmClient) {
        this.cosmos = cosmos;
    }

    public async getMixnodes(page: number, perPage: number): Promise<MixNodesResponse> {
        return {
            nodes: [],
            totalPages: 1,
            totalCount: 0,
            currentPage: page,
            perPage: perPage,
        };
    }

}
