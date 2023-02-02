import { MixNodeRewarding, MixOwnershipResponse, PagedMixDelegationsResponse, PagedMixNodeBondResponse, PagedMixNodeDetailsResponse, UnbondedMixnodeResponse } from "../../compiledTypes";
import expect from 'expect';
import { TestHelper } from "./client";
import { mixnet, mixnodeowneraddress, mix_id, mix_identity } from "./testData";


describe("Mixnode mock tests", () => {
    let testHelper = new TestHelper();

    it("get Mixnode Bonds", () => {
        let execute = testHelper.tests("getMixNodeBonds", [mixnet], <PagedMixNodeBondResponse>{
            nodes: [],
            per_page: 25,
        });
        expect(execute).toBeTruthy();
    });

    it("get Mixnode Delegations Paged", () => {
        let execute = testHelper.tests("getMixNodeDelegationsPaged", [mixnet, mix_identity], <PagedMixDelegationsResponse>{
            delegations: [],
            per_page: 25,
        });
        expect(execute).toBeTruthy();
    });


    it("get Mixnodes Detailed", () => {
        let execute = testHelper.tests("getMixNodesDetailed", [mixnet], <PagedMixNodeDetailsResponse>{
            nodes: [],
            per_page: 25,
        });
        expect(execute).toBeTruthy();
    });

    it("get Mixnode Rewarding Details", () => {
        let execute = testHelper.tests("getMixnodeRewardingDetails", [mixnet, mix_id], <MixNodeRewarding>{
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

    it("get Owned Mixnode", () => {
        let execute = testHelper.tests("getOwnedMixnode", [mixnet, mixnodeowneraddress], <MixOwnershipResponse>{
            address: "",
            mixnode: {}
        });
        expect(execute).toBeTruthy();
    });

    it("get Unbonded Mixnode Information", () => {
        let execute = testHelper.tests("getUnbondedMixNodeInformation", [mixnet, mix_id], <UnbondedMixnodeResponse>{

        });
        expect(execute).toBeTruthy();
    });
});
