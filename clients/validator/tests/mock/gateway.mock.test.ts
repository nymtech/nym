import { INyxdQuery } from "../../src/query-client";
import { GatewayOwnershipResponse, PagedGatewayResponse } from "../../compiledTypes";
import { Mock } from "moq.ts";
import expect from 'expect';
import { TestHelper } from "./client";

describe("Gateway mock tests", () => {
    let mixnet = "n14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sjyvg3g";
    let gatewayowneraddress = "n1rqqw8km7a0rvf8lr6k8dsdqvvkyn2mglj7xxfm"

    let client: Mock<INyxdQuery>;
    let testHelper = new TestHelper();

    beforeEach(() => {
        client = new Mock<INyxdQuery>();
    });

    it("get Gateways Paged", async () => {
        let execute = await testHelper.tests(client, "getGatewaysPaged", [mixnet], <PagedGatewayResponse>{
            nodes: [],
            per_page: 25
        });
        expect(execute).toBeTruthy();
    });

    it("owns Gateway", async () => {
        let execute = await testHelper.tests(client, "ownsGateway", [mixnet, gatewayowneraddress], <GatewayOwnershipResponse>{
            address: gatewayowneraddress,
            gateway: {}
        });
        expect(execute).toBeTruthy();
    });
});
