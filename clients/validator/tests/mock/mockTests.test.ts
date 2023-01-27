import { INyxdQuery } from "../../src/query-client";
import { ContractStateParams, Delegation, GatewayOwnershipResponse, LayerDistribution, MixnetContractVersion, MixNodeRewarding, MixOwnershipResponse, PagedAllDelegationsResponse, PagedDelegatorDelegationsResponse, PagedGatewayResponse, PagedMixDelegationsResponse, PagedMixNodeBondResponse, PagedMixNodeDetailsResponse, RewardingStatus, StakeSaturationResponse, UnbondedMixnodeResponse } from "../../compiledTypes";
import { Mock, Times } from "moq.ts";
import expect from 'expect';

describe("nym-client mocks", () => {
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
        let supply = <string>{}
        const client = new Mock<INyxdQuery>().setup((nym) => nym.getCirculatingSupply(mixnet)).returns(Promise.resolve(supply));
        const obj = client.object();
        let execute = await obj.getCirculatingSupply(mixnet);
        client.verify(
            (nym) => nym.getCirculatingSupply(mixnet),
            Times.Exactly(1)
        );
        expect(execute).toStrictEqual(supply);
        expect(execute).toBeTruthy();
    });

    it("get delegation details", async () => {
        let delegation = <Delegation>{
            owner: mixnodeowneraddress,
            mix_id: mix_id,
            amount: {
                denom: "nym",
                amount: "10"
            },
            height: 1314134144132n,
            proxy: "null"
        }
        const client = new Mock<INyxdQuery>().setup((nym) => nym.getDelegationDetails(mixnet, mix_identity, mixnodeowneraddress)).returns(Promise.resolve(delegation));
        const obj = client.object();
        let execute = await obj.getDelegationDetails(mixnet, mix_identity, mixnodeowneraddress);
        client.verify(
            (nym) => nym.getDelegationDetails(mixnet, mix_identity, mixnodeowneraddress),
            Times.Exactly(1)
        );
        expect(execute).toStrictEqual(delegation);
        expect(execute).toBeTruthy();
    });

    it("get all delegations paged", async () => {
        let delegation = <PagedAllDelegationsResponse>{
            delegations: [],
        }
        const client = new Mock<INyxdQuery>().setup((nym) => nym.getAllDelegationsPaged(mixnet)).returns(Promise.resolve(delegation));
        const obj = client.object();
        let execute = await obj.getAllDelegationsPaged(mixnet);
        client.verify(
            (nym) => nym.getAllDelegationsPaged(mixnet),
            Times.Exactly(1)
        );
        expect(execute).toStrictEqual(delegation);
        expect(execute).toBeTruthy();
    });

    it("get delegator delegations paged", async () => {
        let delegation = <PagedDelegatorDelegationsResponse>{
            delegations: [],
        }
        const client = new Mock<INyxdQuery>().setup((nym) => nym.getDelegatorDelegationsPaged(mixnet, mixnodeowneraddress)).returns(Promise.resolve(delegation));
        const obj = client.object();
        let execute = await obj.getDelegatorDelegationsPaged(mixnet, mixnodeowneraddress);
        client.verify(
            (nym) => nym.getDelegatorDelegationsPaged(mixnet, mixnodeowneraddress),
            Times.Exactly(1)
        );
        expect(execute).toStrictEqual(delegation);
        expect(execute).toBeTruthy();
    });

    it("get gateways paged", async () => {
        let gateway = <PagedGatewayResponse>{
            gateway: [],
            per_page: 25
        }
        const client = new Mock<INyxdQuery>().setup((nym) => nym.getGatewaysPaged(mixnet)).returns(Promise.resolve(gateway));
        const obj = client.object();
        let execute = await obj.getGatewaysPaged(mixnet);
        client.verify(
            (nym) => nym.getGatewaysPaged(mixnet),
            Times.Exactly(1)
        );
        expect(execute).toStrictEqual(gateway);
        expect(execute).toBeTruthy();
    });

    it("get interval reward percent", async () => {
        let percent = <number>{}
        const client = new Mock<INyxdQuery>().setup((nym) => nym.getIntervalRewardPercent(mixnet)).returns(Promise.resolve(percent));
        const obj = client.object();
        let execute = await obj.getIntervalRewardPercent(mixnet);
        client.verify(
            (nym) => nym.getIntervalRewardPercent(mixnet),
            Times.Exactly(1)
        );
        expect(execute).toStrictEqual(percent);
        expect(execute).toBeTruthy();
    });

    it("get layer distribution", async () => {
        let layer = <LayerDistribution>{
            gateways: 10,
            layer1: 2,
            layer2: 2,
            layer3: 5
        }
        const client = new Mock<INyxdQuery>().setup((nym) => nym.getLayerDistribution(mixnet)).returns(Promise.resolve(layer));
        const obj = client.object();
        let execute = await obj.getLayerDistribution(mixnet);
        client.verify(
            (nym) => nym.getLayerDistribution(mixnet),
            Times.Exactly(1)
        );
        expect(execute).toStrictEqual(layer);
        expect(execute).toBeTruthy();
    });

    it("get mixnode bonds", async () => {
        let bonds = <PagedMixNodeBondResponse>{
            nodes: [],
            per_page: 25,
        }
        const client = new Mock<INyxdQuery>().setup((nym) => nym.getMixNodeBonds(mixnet)).returns(Promise.resolve(bonds));
        const obj = client.object();
        let execute = await obj.getMixNodeBonds(mixnet);
        client.verify(
            (nym) => nym.getMixNodeBonds(mixnet),
            Times.Exactly(1)
        );
        expect(execute).toStrictEqual(bonds);
        expect(execute).toBeTruthy();
    });

    it("get mixnode delegations", async () => {
        let delegations = <PagedMixDelegationsResponse>{
            delegations: [],
            per_page: 25,
        }
        const client = new Mock<INyxdQuery>().setup((nym) => nym.getMixNodeDelegationsPaged(mixnet, mix_identity)).returns(Promise.resolve(delegations));
        const obj = client.object();
        let execute = await obj.getMixNodeDelegationsPaged(mixnet, mix_identity);
        client.verify(
            (nym) => nym.getMixNodeDelegationsPaged(mixnet, mix_identity),
            Times.Exactly(1)
        );
        expect(execute).toStrictEqual(delegations);
        expect(execute).toBeTruthy();
    });

    it("get mixnode details", async () => {
        let details = <PagedMixNodeDetailsResponse>{
            nodes: [],
            per_page: 25,
        }
        const client = new Mock<INyxdQuery>().setup((nym) => nym.getMixNodesDetailed(mixnet)).returns(Promise.resolve(details));
        const obj = client.object();
        let execute = await obj.getMixNodesDetailed(mixnet);
        client.verify(
            (nym) => nym.getMixNodesDetailed(mixnet),
            Times.Exactly(1)
        );
        expect(execute).toStrictEqual(details);
        expect(execute).toBeTruthy();
    });

    it("get mixnode rewarding details", async () => {
        let rewarding = <MixNodeRewarding>{
            cost_params: {},
            operator: "",
            delegates: "",
            total_unit_reward: "",
            unit_delegation: "",
            last_rewarded_epoch: 1,
            unique_delegations: 1,
        }
        const client = new Mock<INyxdQuery>().setup((nym) => nym.getMixnodeRewardingDetails(mixnet, mix_id)).returns(Promise.resolve(rewarding));
        const obj = client.object();
        let execute = await obj.getMixnodeRewardingDetails(mixnet, mix_id);
        client.verify(
            (nym) => nym.getMixnodeRewardingDetails(mixnet, mix_id),
            Times.Exactly(1)
        );
        expect(execute).toStrictEqual(rewarding);
        expect(execute).toBeTruthy();
    });

    it("get own mixnode", async () => {
        let own = <MixOwnershipResponse>{
            address: "",
            mixnode: {}
        }
        const client = new Mock<INyxdQuery>().setup((nym) => nym.getOwnedMixnode(mixnet, mixnodeowneraddress)).returns(Promise.resolve(own));
        const obj = client.object();
        let execute = await obj.getOwnedMixnode(mixnet, mixnodeowneraddress);
        client.verify(
            (nym) => nym.getOwnedMixnode(mixnet, mixnodeowneraddress),
            Times.Exactly(1)
        );
        expect(execute).toStrictEqual(own);
        expect(execute).toBeTruthy();
    });

    it("get reward params", async () => {
        let reward = <string>{}
        const client = new Mock<INyxdQuery>().setup((nym) => nym.getRewardParams(mixnet)).returns(Promise.resolve(reward));
        const obj = client.object();
        let execute = await obj.getRewardParams(mixnet);
        client.verify(
            (nym) => nym.getRewardParams(mixnet),
            Times.Exactly(1)
        );
        expect(execute).toStrictEqual(reward);
        expect(execute).toBeTruthy();
    });

    it("get rewarding status", async () => {
        let status = <RewardingStatus>{
            Complete: {},
        }
        const client = new Mock<INyxdQuery>().setup((nym) => nym.getRewardingStatus(mixnet, mix_identity, rewardingIntervalNonce)).returns(Promise.resolve(status));
        const obj = client.object();
        let execute = await obj.getRewardingStatus(mixnet, mix_identity, rewardingIntervalNonce);
        client.verify(
            (nym) => nym.getRewardingStatus(mixnet, mix_identity, rewardingIntervalNonce),
            Times.Exactly(1)
        );
        expect(execute).toStrictEqual(status);
        expect(execute).toBeTruthy();
    });

    it("get stake saturation", async () => {
        let stake = <StakeSaturationResponse>{
            saturation: "",
            uncapped_saturation: "",
            as_at: 1n
        }
        const client = new Mock<INyxdQuery>().setup((nym) => nym.getStakeSaturation(mixnet, mix_id)).returns(Promise.resolve(stake));
        const obj = client.object();
        let execute = await obj.getStakeSaturation(mixnet, mix_id);
        client.verify(
            (nym) => nym.getStakeSaturation(mixnet, mix_id),
            Times.Exactly(1)
        );
        expect(execute).toStrictEqual(stake);
        expect(execute).toBeTruthy();
    });

    it("get state params", async () => {
        let state = <ContractStateParams>{
            minimum_mixnode_pledge: "",
            minimum_gateway_pledge: "",
            mixnode_rewarded_set_size: 240,
            mixnode_active_set_size: 240,
        }
        const client = new Mock<INyxdQuery>().setup((nym) => nym.getStateParams(mixnet)).returns(Promise.resolve(state));
        const obj = client.object();
        let execute = await obj.getStateParams(mixnet);
        client.verify(
            (nym) => nym.getStateParams(mixnet),
            Times.Exactly(1)
        );
        expect(execute).toStrictEqual(state);
        expect(execute).toBeTruthy();
    });

    it("get unbonded node details", async () => {
        let mixnode = <UnbondedMixnodeResponse>{
            //Fix this
        }
        const client = new Mock<INyxdQuery>().setup((nym) => nym.getUnbondedMixNodeInformation(mixnet, mix_id)).returns(Promise.resolve(mixnode));
        const obj = client.object();
        let execute = await obj.getUnbondedMixNodeInformation(mixnet, mix_id);
        client.verify(
            (nym) => nym.getUnbondedMixNodeInformation(mixnet, mix_id),
            Times.Exactly(1)
        );
        expect(execute).toStrictEqual(mixnode);
        expect(execute).toBeTruthy();
    });

    it("get own gateway", async () => {
        let gateway = <GatewayOwnershipResponse>{
            address: gatewayowneraddress,
            gateway: {}
        }
        const client = new Mock<INyxdQuery>().setup((nym) => nym.ownsGateway(mixnet, gatewayowneraddress)).returns(Promise.resolve(gateway));
        const obj = client.object();
        let execute = await obj.ownsGateway(mixnet, gatewayowneraddress);
        client.verify(
            (nym) => nym.ownsGateway(mixnet, gatewayowneraddress),
            Times.Exactly(1)
        );
        expect(execute).toStrictEqual(gateway);
        expect(execute).toBeTruthy();
    });
});

const sleep = (ms) => new Promise(r => setTimeout(r, ms));