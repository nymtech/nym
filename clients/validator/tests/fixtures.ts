import { coins } from "@cosmjs/launchpad";
import {PagedGatewayResponse, PagedMixnodeResponse} from "../src/signing-client";
import {GatewayBond, MixNodeBond} from "../src/types"

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

    export class Gateways {
        static single(): GatewayBond {
            return {
                amount: coins(666, "unym"),
                owner: "bob",
                gateway: {
                    mix_host: "1.1.1.1:1234",
                    clients_host: "ws://1.1.1.1:1235",
                    location: "London, UK",
                    identity_key: "bar",
                    sphinx_key: "foo",
                    version: "0.10.0",
                }
            };
        }

        static list1(): GatewayBond[] {
            return [Gateways.single()]
        }

        static list2(): GatewayBond[] {
            return [Gateways.single(), Gateways.single()]
        }

        static list3(): GatewayBond[] {
            return [Gateways.single(), Gateways.single(), Gateways.single()]
        }

        static list4(): GatewayBond[] {
            return [Gateways.single(), Gateways.single(), Gateways.single(), Gateways.single()]
        }
    }

    export class GatewaysResp {
        static empty(): PagedGatewayResponse {
            return {
                nodes: [],
                per_page: 2,
                start_next_after: "",
            }
        }

        static onePage(): PagedGatewayResponse {
            return {
                nodes: Gateways.list2(),
                per_page: 2,
                start_next_after: "",
            }
        }

        static page1of2(): PagedGatewayResponse {
            return {
                nodes: Gateways.list2(),
                per_page: 2,
                start_next_after: "2"
            }
        }

        static halfPage2of2(): PagedGatewayResponse {
            return {
                nodes: Gateways.list1(),
                per_page: 2,
                start_next_after: "",

            }
        }

        static fullPage2of2(): PagedGatewayResponse {
            return {
                nodes: Gateways.list2(),
                per_page: 2,
                start_next_after: "",
            }
        }
    }
}