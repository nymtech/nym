import MixnodesCache from "./caches/mixnodes";

export interface MixNode {
    stake: number,
    pubKey: string,
    layer: number,
}

export interface MixNodesResponse {
    nodes: MixNode[],
    currentPage: number,
    perPage: number,
    totalCount: number,
    totalPages: number,
}