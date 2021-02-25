import { MixNode } from "../types";
import { INetClient, PagedResponse } from "../net-client"
import { log } from "../utils"

export { MixnodesCache };

export default class MixnodesCache {
    mixNodes: MixNode[]
    netClient: INetClient
    perPage: number

    constructor(netClient: INetClient, perPage: number) {
        this.netClient = netClient;
        this.mixNodes = [];
        this.perPage = perPage;
    }

    async refreshMixNodes() {
        let response: PagedResponse;
        let start_after;
        do {
            response = await this.netClient.getMixNodes(this.perPage, start_after);
            response.nodes.forEach(node => this.mixNodes.push(node));
            start_after = response.start_next_after;
        } while (start_after != null)
    }
}