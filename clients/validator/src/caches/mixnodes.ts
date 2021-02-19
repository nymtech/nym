import NetClient from "../net-client";
import { MixNode } from "../types";
import { INetClient } from "../net-client"

export { MixnodesCache };

export default class MixnodesCache {
    mixNodes: MixNode[]
    netClient: INetClient
    perPage: number

    constructor(netClient: INetClient) {
        this.netClient = netClient;
        this.mixNodes = [];
        this.perPage = 100; // this can probably be set in the constructor
    }

    refreshMixNodes() {
        this.mixNodes = this.netClient.getMixnodes(1, this.perPage)
    }
}