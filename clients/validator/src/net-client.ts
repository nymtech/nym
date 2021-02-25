import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { MixNode } from './types'
import { connect as connectHelper } from "./stargate-helper";


export interface INetClient {
    getMixNodes(): Promise<PagedResponse>;
}

// interface INetClientStatic {
//     connect(contractAddress: string, mnemonic: string, url: string): Promise<INetClient>;
// }

export default class NetClient implements INetClient {
    private clientAddress: string;
    private cosmClient: SigningCosmWasmClient;

    private constructor(clientAddress: string, cosmClient: SigningCosmWasmClient) {
        this.clientAddress = clientAddress;
        this.cosmClient = cosmClient;
    }

    public static async connect(contractAddress: string, mnemonic: string, url: string): Promise<INetClient> {
        let { client, address } = await connectHelper(mnemonic, {});
        let netClient = new NetClient(address, client);
        return netClient;
    }

    public getMixNodes(): Promise<PagedResponse> {
        return this.cosmClient.queryContractSmart(this.clientAddress, { get_mix_nodes: {} });
    }
}

export interface PagedResponse {
    nodes: MixNode[],
    per_page: number, // TODO: camelCase
    start_next_after: string, // TODO: camelCase
}