import { MixNodeBond, PagedMixnodeResponse } from "../types";
import { INetClient } from "../net-client";
import { IQueryClient } from "../query-client";
import { VALIDATOR_API_MIXNODES, VALIDATOR_API_PORT } from "../index";
import axios from "axios";

export { MixnodesCache };

/**
 * There are serious limits in smart contract systems, but we need to keep track of
 * potentially thousands of nodes. MixnodeCache instances repeatedly make requests for
 *  paged data about what mixnodes exist, and keep them locally in memory so that they're
 *  available for querying.
 *  */
export default class MixnodesCache {
  mixNodes: MixNodeBond[];
  client: INetClient | IQueryClient;
  perPage: number;

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
      response = await this.client.getMixNodes(
        contractAddress,
        this.perPage,
        next
      );
      newMixnodes = newMixnodes.concat(response.nodes);
      next = response.start_next_after;
      // if `start_next_after` is not set, we're done
      if (!next) {
        break;
      }
    }

    this.mixNodes = newMixnodes;
    return this.mixNodes;
  }

  /// Makes  requests to assemble a full list of mixnodes from validator-api
  async refreshValidatorAPIMixNodes(urls: string[]): Promise<MixNodeBond[]> {
    for (const url of urls) {
      const validator_api_url = new URL(url);
      validator_api_url.port = VALIDATOR_API_PORT;
      validator_api_url.pathname += VALIDATOR_API_MIXNODES;
      const response = await axios.get(validator_api_url.toString());
      if (response.status == 200) {
        return response.data;
      }
    }
    throw new Error("None of the provided validators seem to be alive");
  }
}
