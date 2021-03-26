import { MixNodeBond } from "../types";
import { INetClient } from "../net-client"
import {IQueryClient} from "../query-client";
import {PagedResponse} from "../index";

export { MixnodesCache };

/**
 * There are serious limits in smart contract systems, but we need to keep track of 
 * potentially thousands of nodes. MixnodeCache instances repeatedly make requests for
 *  paged data about what mixnodes exist, and keep them locally in memory so that they're
 *  available for querying.
 *  */
export default class MixnodesCache {
    mixNodes: MixNodeBond[]
    client: INetClient | IQueryClient
    perPage: number

    constructor(client: INetClient | IQueryClient, perPage: number) {
        this.client = client;
        this.mixNodes = [];
        this.perPage = perPage;
    }

    /// Makes repeated requests to assemble a full list of nodes. 
    /// Requests continue to be make as long as `shouldMakeAnotherRequest()`
    // returns true. 
    async refreshMixNodes(contractAddress: string): Promise<MixNodeBond[]> {
        this.mixNodes = [];
        let response: PagedResponse;
        let next: string | undefined;
        do {
            response = await this.client.getMixNodes(contractAddress, this.perPage, next);
            response.nodes.forEach(node => this.mixNodes.push(node));
            next = response.start_next_after;
        } while (this.shouldMakeAnotherRequest(response))
        return this.mixNodes;
    }

    /// The paging interface on the smart contracts is a bit gross at the moment.
    /// This returns `true` if the `start_next_after` property of the response is set
    /// and the page we've just got back is the same length as perPage on this
    /// NetClient instance (we don't have any idea whether there is a next page
    /// so if both these things are true we should make another request);
    /// otherwise returns false.
    shouldMakeAnotherRequest(response: PagedResponse): boolean {
        const next = response.start_next_after;
        const nextExists: boolean = (next !== null && next !== undefined && next !== "");
        const fullPage: boolean = response.nodes.length == this.perPage;
        return fullPage && nextExists;
    }
}