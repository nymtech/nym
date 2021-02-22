import { MixNode, MixNodesResponse } from '../src/types'

export namespace Fixtures {

    export class MixNodes {
        static single(): MixNode {
            return {
                stake: 1,
                layer: 1,
                pubKey: "foo"
            };
        }

        static list2(): MixNode[] {
            return [MixNodes.single(), MixNodes.single()]
        }

        static list3(): MixNode[] {
            return [MixNodes.single(), MixNodes.single(), MixNodes.single()]
        }

    }

    export class MixNodesResp {
        static empty(): MixNodesResponse {
            return {
                nodes: [],
                perPage: 2,
                totalPages: 1,
                currentPage: 1,
                totalCount: 0,
            }
        }

        static onePage(): MixNodesResponse {
            return {
                nodes: MixNodes.list2(),
                perPage: 2,
                totalPages: 1,
                currentPage: 1,
                totalCount: 2,
            }
        }

        static partial(): MixNodesResponse {
            return {
                nodes: MixNodes.list2(),
                perPage: 2,
                totalPages: 2,
                currentPage: 1,
                totalCount: 3,
            }
        }
    }
}