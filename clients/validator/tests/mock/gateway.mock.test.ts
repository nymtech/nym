import { GatewayOwnershipResponse, PagedGatewayResponse } from "../../compiledTypes";
import expect from 'expect';
import { TestHelper } from "./client";
import { ownGateway, pagedGateway } from "../../types/expectedResponses";
import {gatewayowneraddress, mixnet} from "./testData";

describe("Gateway mock tests", () => {

    let testHelper = new TestHelper();

    it("get Gateways Paged", () => {
        let execute = testHelper.tests("getGatewaysPaged", [mixnet], 
        // pagedGateway
        <PagedGatewayResponse>{
            nodes: [],
            per_page: 25
        }
        );
        expect(execute).toBeTruthy();
    });

    it("owns Gateway", () => {
        let execute = testHelper.tests("ownsGateway", [mixnet, gatewayowneraddress], 
        // ownGateway
        <GatewayOwnershipResponse>{
            address: gatewayowneraddress,
            gateway: {}
        }
        );
        expect(execute).toBeTruthy();
    });
});
