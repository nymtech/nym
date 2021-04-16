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
        for (;;) {
            response = await this.client.getGateways(contractAddress, this.perPage, next);
            newGateways = newGateways.concat(response.nodes)
            next = response.start_next_after;
            // if `start_next_after` is not set, we're done
            if (!next) {
                break
            }
        }

        this.gateways = newGateways
        return newGateways;
    }
}