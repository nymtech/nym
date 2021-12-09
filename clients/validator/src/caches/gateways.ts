import {GatewayBond, PagedGatewayResponse} from "../types";
import {ISigningClient} from "../signing-client"
import {IQueryClient} from "../query-client";
import {VALIDATOR_API_GATEWAYS, VALIDATOR_API_PORT} from "../index";
import axios from "axios";


/**
 * There are serious limits in smart contract systems, but we need to keep track of
 * potentially thousands of nodes. GatewaysCache instances repeatedly make requests for
 * paged data about what gateways exist, and keep them locally in memory so that they're
 * available for querying.
 **/
export default class GatewaysCache {
    gateways: GatewayBond[]
    client: ISigningClient | IQueryClient
    perPage: number

    constructor(client: ISigningClient | IQueryClient, perPage: number) {
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

    /// Makes requests to assemble a full list of gateways from validator-api
    async refreshValidatorAPIGateways(url: string): Promise<GatewayBond[]> {
        const validator_api_url = new URL(url);
        validator_api_url.port = VALIDATOR_API_PORT;
        validator_api_url.pathname += VALIDATOR_API_GATEWAYS;
        const response = await axios.get(validator_api_url.toString());
        if (response.status == 200) {
            return response.data;
        }

        throw new Error("None of the provided validators seem to be alive")
    }
}