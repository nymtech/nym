import { INyxdQuery } from "../../src/query-client";
import { MixNodeRewarding, MixOwnershipResponse, PagedMixDelegationsResponse, PagedMixNodeBondResponse, PagedMixNodeDetailsResponse, UnbondedMixnodeResponse } from "../../compiledTypes";
import { Mock } from "moq.ts";
import expect from 'expect';
import { TestHelper } from "./client";

describe("Mixnode mock tests", () => {
    let mixnet = "n14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sjyvg3g";
    let mix_id = 436207616;
    let mix_identity = "26";
    let mixnodeowneraddress = "n1fzv4jc7fanl9s0qj02ge2ezk3kts545kjtek47";

    let client: Mock<INyxdQuery>;
    let testHelper = new TestHelper();

    beforeEach(() => {
        client = new Mock<INyxdQuery>();
    });

    it("get Mixnode Bonds", async () => {
        let execute = await testHelper.tests(client, "getMixNodeBonds", [mixnet], <PagedMixNodeBondResponse>{
            nodes: [],
            per_page: 25,
        });
        expect(execute).toBeTruthy();
    });

    it("get Mixnode Delegations Paged", async () => {
        let execute = await testHelper.tests(client, "getMixNodeDelegationsPaged", [mixnet, mix_identity], <PagedMixDelegationsResponse>{
            delegations: [],
            per_page: 25,
        });
        expect(execute).toBeTruthy();
    });


    it("get Mixnodes Detailed", async () => {
        let execute = await testHelper.tests(client, "getMixNodesDetailed", [mixnet], <PagedMixNodeDetailsResponse>{
            nodes: [],
            per_page: 25,
        });
        expect(execute).toBeTruthy();
    });

    it("get Mixnode Rewarding Details", async () => {
        let execute = await testHelper.tests(client, "getMixnodeRewardingDetails", [mixnet, mix_id], <MixNodeRewarding>{
            cost_params: {},
            operator: "",
            delegates: "",
            total_unit_reward: "",
            unit_delegation: "",
            last_rewarded_epoch: 1,
            unique_delegations: 1,
        });
        expect(execute).toBeTruthy();
    });

    it("get Owned Mixnode", async () => {
        let execute = await testHelper.tests(client, "getOwnedMixnode", [mixnet, mixnodeowneraddress], <MixOwnershipResponse>{
            address: "",
            mixnode: {}
        });
        expect(execute).toBeTruthy();
    });

    it("get Unbonded Mixnode Information", async () => {
        let execute = await testHelper.tests(client, "getUnbondedMixNodeInformation", [mixnet, mix_id], <UnbondedMixnodeResponse>{

        });
        expect(execute).toBeTruthy();
    });
});
