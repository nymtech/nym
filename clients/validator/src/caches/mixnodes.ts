import { MixNode } from "../types";
import { INetClient, PagedResponse } from "../net-client"

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
        const firstPage = await this.netClient.getMixNodes();
        this.mixNodes = firstPage.nodes;

        // if (firstPage.totalPages > 1) {
        //     const responses = await this.makeAdditionalPagedRequests(firstPage);
        //     responses.forEach(response => {
        //         this.mixNodes = [...this.mixNodes, ...response.nodes];
        //     });
        // }
    }

    // async makeAdditionalPagedRequests(firstPage: PagedResponse): Promise<PagedResponse[]> {
    //     const additionalRequests = [];
    //     const numRequests = firstPage.totalPages - 1;
    //     let nextPage = 2;
    //     for (let i = 0; i < numRequests; i++) {
    //         const req = await this.netClient.getMixnodes(nextPage, this.perPage);
    //         additionalRequests.push(req);
    //         nextPage++;
    //     }
    //     return await Promise.all(additionalRequests)
    // }
}