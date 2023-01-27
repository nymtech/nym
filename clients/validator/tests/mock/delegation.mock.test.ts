import { INyxdQuery } from "../../src/query-client";
import { Delegation, GatewayOwnershipResponse, PagedAllDelegationsResponse, PagedDelegatorDelegationsResponse, PagedGatewayResponse } from "../../compiledTypes";
import { Mock } from "moq.ts";
import expect from 'expect';
import { TestHelper } from "./client";

describe("Delegation mock tests", () => {
    let mixnet = "n14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sjyvg3g";
    let mix_id = 436207616;
    let mix_identity = "26";
    let mixnodeowneraddress = "n1fzv4jc7fanl9s0qj02ge2ezk3kts545kjtek47";
    let gatewayowneraddress = "n1rqqw8km7a0rvf8lr6k8dsdqvvkyn2mglj7xxfm"

    let client: Mock<INyxdQuery>;
    let testHelper = new TestHelper();

    beforeEach(() => {
        client = new Mock<INyxdQuery>();
    });

    it("get Delegation Details", async () => {
        let execute = testHelper.tests(client, "getDelegationDetails", [mixnet, mix_identity, mixnodeowneraddress], <Delegation>{
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

    it("get All Delegations Paged", async () => {
        let execute = testHelper.tests(client, "getAllDelegationsPaged", [mixnet], <PagedAllDelegationsResponse>{
            delegations: [],
        });
        expect(execute).toBeTruthy();
    });

    it("get Delegator Delegations Paged", async () => {
        let execute = testHelper.tests(client, "getDelegatorDelegationsPaged", [mixnet, mixnodeowneraddress], <PagedDelegatorDelegationsResponse>{
            delegations: [],
        });
        expect(execute).toBeTruthy();
    });

    it("get Gateways Paged", async () => {
        let execute = testHelper.tests(client, "getGatewaysPaged", [mixnet], <PagedGatewayResponse>{
            gateway: [],
            per_page: 25
        });
        expect(execute).toBeTruthy();
    });

    it("owns Gateway", async () => {
        let execute = testHelper.tests(client, "ownsGateway", [mixnet, gatewayowneraddress], <GatewayOwnershipResponse>{
            address: gatewayowneraddress,
            gateway: {}
        });
        expect(execute).toBeTruthy();
    });
});
