import { INyxdQuery } from "../../src/query-client";
import { MixnetContractVersion } from "../../compiledTypes";
import { Mock, Times } from "moq.ts";
import expect from 'expect';

describe("nym-client mocks", () => {
    // To-Do: Add beforeAll function 
    // To-Do: Add separate file with data 
    // To-Do: Add some assertions 
    const client = new Mock<INyxdQuery>();
    let mixnet = "n14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sjyvg3g";
    let mix_id = 436207616;
    let mix_identity = "26";
    let mixnodeowneraddress = "n1fzv4jc7fanl9s0qj02ge2ezk3kts545kjtek47";
    let rewardingIntervalNonce = 1;
    let gatewayowneraddress = "n1rqqw8km7a0rvf8lr6k8dsdqvvkyn2mglj7xxfm"

    it("get mixnet contract version data", async () => {
        
        //build the expected response type
        let mix = <MixnetContractVersion>{
            build_timestamp: "test",
            commit_branch: "test",
            build_version: "test",
            rustc_version: "test",
            commit_sha: "test",
            commit_timestamp: "test"
        }
        
        //buld the client and expect the response
        const client = new Mock<INyxdQuery>().setup((nym) => nym.getContractVersion(mixnet)).returns(Promise.resolve(mix));
        
        const obj = client.object();
        
        //execute the method
        let execute = await obj.getContractVersion(mixnet);
        
        client.verify(
            (nym) => nym.getContractVersion(mixnet),
            Times.Exactly(1)
        );
        
        expect(execute).toStrictEqual(mix);
        expect(execute).toBeTruthy();
    });

    it("get circulating supply", async () => {
        const supply = await client.setup((nym) => nym.getCirculatingSupply(mixnet));
        expect(supply).toBeTruthy();
    });

    it("get delegation details", async () => {
        const delegation = await client.setup((nym) => nym.getDelegationDetails(mixnet, mix_identity, mixnodeowneraddress));
        expect(delegation).toBeTruthy();
    });

    it("get all delegations paged", async () => {
        const delegation = await client.setup((nym) => nym.getAllDelegationsPaged(mixnet));
        expect(delegation).toBeTruthy();
    });

    it("get delegator delegations paged", async () => {
        const delegation = await client.setup((nym) => nym.getDelegatorDelegationsPaged(mixnet, mixnodeowneraddress));
        expect(delegation).toBeTruthy();
    });

    it("get gateways paged", async () => {
        const gateways = await client.setup((nym) => nym.getGatewaysPaged(mixnet));
        expect(gateways).toBeTruthy();
    });

    it("get interval reward percent", async () => {
        const interval = await client.setup((nym) => nym.getIntervalRewardPercent(mixnet));
        expect(interval).toBeTruthy();
    });

    it("get layer distribution", async () => {
        const layer = await client.setup((nym) => nym.getLayerDistribution(mixnet));
        expect(layer).toBeTruthy();
    });

    it("get mixnode bonds", async () => {
        const mixnode = await client.setup((nym) => nym.getMixNodeBonds(mixnet));
        expect(mixnode).toBeTruthy();
    });

    it("get mixnode delegations", async () => {
        const mixnode = await client.setup((nym) => nym.getMixNodeDelegationsPaged(mixnet, mix_identity));
        expect(mixnode).toBeTruthy();
    });

    it("get mixnode details", async () => {
        const mixnode = await client.setup((nym) => nym.getMixNodesDetailed(mixnet));
        expect(mixnode).toBeTruthy();
    });

    it("get mixnode rewarding details", async () => {
        const mixnode = await client.setup((nym) => nym.getMixnodeRewardingDetails(mixnet, mix_id));
        expect(mixnode).toBeTruthy();
    });

    it("get own mixnode", async () => {
        const mixnode = await client.setup((nym) => nym.getOwnedMixnode(mixnet, mixnodeowneraddress));
        expect(mixnode).toBeTruthy();
    });

    it("get reward params", async () => {
        const reward = await client.setup((nym) => nym.getRewardParams(mixnet));
        expect(reward).toBeTruthy();
    });

    it("get rewarding status", async () => {
        const status = await client.setup((nym) => nym.getRewardingStatus(mixnet, mix_identity, rewardingIntervalNonce));
        expect(status).toBeTruthy();
    });

    it("get stake saturation", async () => {
        const stake = await client.setup((nym) => nym.getStakeSaturation(mixnet, mix_id));
        expect(stake).toBeTruthy();
    });

    it("get state params", async () => {
        const state = await client.setup((nym) => nym.getStateParams(mixnet));
        expect(state).toBeTruthy();
    });

    it("get unbonded node details", async () => {
        const mixnode = await client.setup((nym) => nym.getUnbondedMixNodeInformation(mixnet, mix_id));
        expect(mixnode).toBeTruthy();
    });

    it("get own gateway", async () => {
        const gateway = await client.setup((nym) => nym.ownsGateway(mixnet, gatewayowneraddress));
        expect(gateway).toBeTruthy();
    });
});


const sleep = (ms) => new Promise(r => setTimeout(r, ms));