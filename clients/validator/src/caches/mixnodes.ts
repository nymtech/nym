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
        if (firstPage.totalPages > 1) {
            const responses = await this.makeAdditionalPagedRequests(firstPage).then(responses => {
                responses.forEach(response => {
                    console.log(`HEY FUCKER ${response}`);
                    this.mixNodes.concat(response.nodes);
                });

            });
        } else {
            this.mixNodes = firstPage.nodes;
        }
    }

    async makeAdditionalPagedRequests(firstPage: MixNodesResponse): Promise<MixNodesResponse[]> {
        const additionalRequests = [];
        const numRequests = firstPage.totalPages - 1;
        let nextPage = 2;
        for (let i = 0; i < numRequests; i++) {
            const responsePromise = await this.netClient.getMixnodes(nextPage, this.perPage);
            additionalRequests.push(responsePromise);
            nextPage++;
        }
        return await Promise.all(additionalRequests)
    }
}