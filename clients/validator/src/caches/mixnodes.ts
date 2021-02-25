import { MixNode } from "../types";
import { INetClient, PagedResponse } from "../net-client"

export { MixnodesCache };

export default class MixnodesCache {
    mixNodes: MixNode[]
    netClient: INetClient
    perPage: number
    requestCount: number;

    constructor(netClient: INetClient, perPage: number) {
        this.netClient = netClient;
        this.mixNodes = [];
        this.perPage = perPage;
        this.requestCount = 0;
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