import { coins } from '@cosmjs/launchpad';
import { PagedResponse } from '../src/net-client';
import { MixNodeBond } from '../src/types'

export namespace Fixtures {
    export class MixNodes {
        static single(): MixNodeBond {
            return {
                amount: coins(666, "unym"),
                owner: "bob",
                mix_node: {
                    host: "1.1.1.1",
                    layer: 1,
                    location: "London, UK",
                    sphinx_key: "foo",
                    version: "0.10.0",
                }
            };
        }

        static list1(): MixNodeBond[] {
            return [MixNodes.single()]
        }

        static list2(): MixNodeBond[] {
            return [MixNodes.single(), MixNodes.single()]
        }

        static list3(): MixNodeBond[] {
            return [MixNodes.single(), MixNodes.single(), MixNodes.single()]
        }

        static list4(): MixNodeBond[] {
            return [MixNodes.single(), MixNodes.single(), MixNodes.single(), MixNodes.single()]
        }
    }

    export class MixNodesResp {
        static empty(): PagedResponse {
            return {
                nodes: [],
                per_page: 2,
                start_next_after: null,
            }
        }

        static onePage(): PagedResponse {
            return {
                nodes: MixNodes.list2(),
                per_page: 2,
                start_next_after: null
            }
        }

        static page1of2(): PagedResponse {
            return {
                nodes: MixNodes.list2(),
                per_page: 2,
                start_next_after: "2"
            }
        }

        static halfPage2of2(): PagedResponse {
            return {
                nodes: MixNodes.list1(),
                per_page: 2,
                start_next_after: null

            }
        }

        static fullPage2of2(): PagedResponse {
            return {
                nodes: MixNodes.list2(),
                per_page: 2,
                start_next_after: null,
            }
        }
    }
}