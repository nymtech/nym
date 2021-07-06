import { MixNodeBond } from "../types";
import { INetClient } from "../net-client"
import {IQueryClient} from "../query-client";
import {PagedMixnodeResponse, VALIDATOR_API_PORT} from "../index";
import axios from "axios";

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
        let newMixnodes: MixNodeBond[] = [];
        let response: PagedMixnodeResponse;
        let next: string | undefined = undefined;
        for (;;) {
            response = await this.client.getMixNodes(contractAddress, this.perPage, next);
            newMixnodes = newMixnodes.concat(response.nodes)
            next = response.start_next_after;
            // if `start_next_after` is not set, we're done
            if (!next) {
                break
            }
        }

        this.mixNodes = newMixnodes
        return this.mixNodes;
    }

    /// Makes  requests to assemble a full list of mixnodes from validator-api
    async refreshValidatorAPIMixNodes(url: string): Promise<MixNodeBond[]> {
        const validator_api_url = url.split(":", 2);
        validator_api_url.push(VALIDATOR_API_PORT);
        const response = await axios.get(validator_api_url.join(":").concat("/v1/mixnodes"));
        return response.data;
    }
}