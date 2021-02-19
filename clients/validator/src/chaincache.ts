import NetClient from "./net-client";
import { MixNode } from "./types";
import { INetClient } from "./net-client"

export { ChainCache };

export default class ChainCache {
    mixNodes: MixNode[]
    netClient: INetClient
    perPage: number

    constructor(netClient: INetClient) {
        this.netClient = netClient;
        this.mixNodes = [];
        this.perPage = 100;
    }

    refreshMixNodes() {
        this.mixNodes = this.netClient.getMixnodes(1, this.perPage)
    }
}