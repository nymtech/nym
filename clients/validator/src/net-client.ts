import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { MixNode } from './types'
import { connect as connectHelper } from "./stargate-helper";


export interface INetClient {
    getMixNodes(limit: number, start_after?: string): Promise<PagedResponse>;
}


/// Takes care of network communication between this code and the validator
/// Depends on `SigningCosmWasClient`, which signs all requests using keypairs
/// derived from on bech32 mnemonics.
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

    public getMixNodes(limit: number, start_after?: string): Promise<PagedResponse> {
        if (start_after == undefined) {
            return this.cosmClient.queryContractSmart(this.clientAddress, { get_mix_nodes: { limit } });
        } else {
            return this.cosmClient.queryContractSmart(this.clientAddress, { get_mix_nodes: { limit, start_after } });
        }
    }
}

/// One page of a possible multi-page set of mixnodes. The paging interface is quite
/// inconvenient, as we don't have the two pieces of information we need to know
/// in order to do paging nicely (namely `currentPage` and `totalPages` parameters). 
///
/// Instead, we have only `start_next_page_after`, i.e. the key of the last record
/// on this page. In order to get the *next* page, CosmWasm looks at that value, 
/// finds the next record, and builds the next page starting there. This happens
/// **in series** rather than **in parallel** (!). 
///
/// So we have some consistency problems: 
///
/// * we can't make requests at a given block height, so the result set
///    which we assemble over time may change while requests are being made.
/// * at some point we will make a request for a `start_next_page_after` key 
///   which has just been deleted from the database.
///
/// TODO: more robust error handling on the "deleted key" case.
export interface PagedResponse {
    nodes: MixNode[],
    per_page: number, // TODO: camelCase
    start_next_after: string, // TODO: camelCase
}