import { Delegation, PagedAllDelegationsResponse, PagedDelegatorDelegationsResponse } from "../../compiledTypes";
import expect from 'expect';
import { TestHelper } from "./client";
import { mixnet, mixnodeowneraddress, mix_id, mix_identity} from "./testData";


describe("Delegation mock tests", () => {
    let testHelper = new TestHelper();

    it("get Delegation Details", () => {
        let execute = testHelper.tests("getDelegationDetails", [mixnet, mix_identity, mixnodeowneraddress], <Delegation>{
            owner: mixnodeowneraddress,
            mix_id: mix_id,
            amount: {
                denom: "nym",
                amount: "10"
            },
            height: 1314134144132n,
            proxy: "null"
        });
        expect(execute).toBeTruthy();
    });

    it("get All Delegations Paged", () => {
        let execute = testHelper.tests("getAllDelegationsPaged", [mixnet], <PagedAllDelegationsResponse>{
            delegations: [],
        });
        expect(execute).toBeTruthy();
    });

    it("get Delegator Delegations Paged", () => {
        let execute = testHelper.tests("getDelegatorDelegationsPaged", [mixnet, mixnodeowneraddress], <PagedDelegatorDelegationsResponse>{
            delegations: [],
        });
        expect(execute).toBeTruthy();
    });
});
