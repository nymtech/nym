import { GatewayBond } from "../types";
import {INetClient} from "../net-client"
import {IQueryClient} from "../query-client";
import {PagedGatewayResponse} from "../index";


/**
 * There are serious limits in smart contract systems, but we need to keep track of
 * potentially thousands of nodes. GatewaysCache instances repeatedly make requests for
 * paged data about what gateways exist, and keep them locally in memory so that they're
 * available for querying.
 **/
export default class GatewaysCache {
    gateways: GatewayBond[]
    client: INetClient | IQueryClient
    perPage: number

    constructor(client: INetClient | IQueryClient, perPage: number) {
        this.client = client;
        this.gateways = [];
        this.perPage = perPage;
    }

    /// Makes repeated requests to assemble a full list of gateways.
    /// Requests continue to be make as long as `shouldMakeAnotherRequest()`
    /// returns true.
    async refreshGateways(contractAddress: string): Promise<GatewayBond[]> {
        let newGateways: GatewayBond[] = [];
        let response: PagedGatewayResponse;
        let next: string | undefined = undefined;
        do {
            response = await this.client.getGateways(contractAddress, this.perPage, next);
            newGateways = newGateways.concat(response.nodes)
            next = response.start_next_after;
        } while (this.shouldMakeAnotherRequest(response))

        this.gateways = newGateways
        return newGateways;
    }

    /// The paging interface on the smart contracts is a bit gross at the moment.
    /// This returns `true` if the `start_next_after` property of the response is set
    /// and the page we've just got back is the same length as perPage on this
    /// NetClient instance (we don't have any idea whether there is a next page
    /// so if both these things are true we should make another request);
    /// otherwise returns false.
    shouldMakeAnotherRequest(response: PagedGatewayResponse): boolean {
        const next = response.start_next_after;
        const nextExists: boolean = (next !== null && next !== undefined && next !== "");
        const fullPage: boolean = response.nodes.length == this.perPage;
        return fullPage && nextExists;
    }
}