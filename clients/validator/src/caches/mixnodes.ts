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
        let next: string | undefined;
        do {
            response = await this.netClient.getMixNodes(this.perPage, next);
            response.nodes.forEach(node => this.mixNodes.push(node));
            next = response.start_next_after;
        } while (this.shouldMakeAnotherRequest(response))
    }
    shouldMakeAnotherRequest(response: PagedResponse): boolean {
        let next = response.start_next_after;
        let nextExists: boolean = (next != null && next != undefined && next != "");
        let fullPage: boolean = response.nodes.length == this.perPage;
        if (fullPage && nextExists) {
            this.requestCount++;
            return true;
        } else {
            return false;
        }
    }
}