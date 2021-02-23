import { MixNode, MixNodesResponse } from "../types";
import { INetClient } from "../net-client"

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
        const firstPage = await this.netClient.getMixnodes(1, this.perPage);
        this.mixNodes = firstPage.nodes;

        if (firstPage.totalPages > 1) {
            const responses = await this.makeAdditionalPagedRequests(firstPage);
            console.log(`response is: ${responses[0].nodes.length}`)
            responses.forEach(response => {
                console.log(`this.mixNodes.length: ${this.mixNodes.length}`)
                this.mixNodes = [...this.mixNodes, ...response.nodes];
                console.log(this.mixNodes.length)
            });
        }
    }

    async makeAdditionalPagedRequests(firstPage: MixNodesResponse): Promise<MixNodesResponse[]> {
        const additionalRequests = [];
        const numRequests = firstPage.totalPages - 1;
        let nextPage = 2;
        for (let i = 0; i < numRequests; i++) {
            const req = await this.netClient.getMixnodes(nextPage, this.perPage);
            additionalRequests.push(req);
            nextPage++;
        }
        return await Promise.all(additionalRequests)
    }
}