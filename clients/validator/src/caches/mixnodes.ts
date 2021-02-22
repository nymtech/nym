import { MixNode } from "../types";
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

    refreshMixNodes() {
        this.mixNodes = this.netClient.getMixnodes(1, this.perPage)
    }
}