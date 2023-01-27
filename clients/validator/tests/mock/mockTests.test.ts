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
    let client: Mock<INyxdQuery>;

    beforeEach(() => {
        client = new Mock<INyxdQuery>();
    });

    const tests = (methodName: string, args: any[], expectedResult: any) => {
        it(methodName, async () => {
            client.setup(nym => nym[methodName](...args)).returns(Promise.resolve(expectedResult));
            let obj = client.object();
            let execute = await obj[methodName](...args);
            client.verify(nym => nym[methodName](...args), Times.Exactly(1));
            expect(execute).toStrictEqual(expectedResult);
            expect(execute).toBeTruthy();
        });
    }

    tests("getContractVersion", [mixnet], <MixnetContractVersion>{
        build_timestamp: "test",
        commit_branch: "test",
        build_version: "test",
        rustc_version: "test",
        commit_sha: "test",
        commit_timestamp: "test"
    });

    tests("getCirculatingSupply", [mixnet], <string>{
    });

    tests("getDelegationDetails", [mixnet, mix_identity, mixnodeowneraddress], <Delegation>{
        owner: mixnodeowneraddress,
        mix_id: mix_id,
        amount: {
            denom: "nym",
            amount: "10"
        },
        height: 1314134144132n,
        proxy: "null"
    });

    tests("getAllDelegationsPaged", [mixnet], <PagedAllDelegationsResponse>{
        delegations: [],
    });

    tests("getDelegatorDelegationsPaged", [mixnet, mixnodeowneraddress], <PagedDelegatorDelegationsResponse>{
        delegations: [],
    });

    tests("getGatewaysPaged", [mixnet], <PagedGatewayResponse>{
        gateway: [],
        per_page: 25
    });

    tests("getIntervalRewardPercent", [mixnet], <number>{
    });

    tests("getLayerDistribution", [mixnet], <LayerDistribution>{
        gateways: 10,
        layer1: 2,
        layer2: 2,
        layer3: 5
    });

    tests("getMixNodeBonds", [mixnet], <PagedMixNodeBondResponse>{
        nodes: [],
        per_page: 25,
    });

    tests("getMixNodeDelegationsPaged", [mixnet, mix_identity], <PagedMixDelegationsResponse>{
        delegations: [],
        per_page: 25,
    });

    tests("getMixNodesDetailed", [mixnet], <PagedMixNodeDetailsResponse>{
        nodes: [],
        per_page: 25,
    });

    tests("getMixnodeRewardingDetails", [mixnet, mix_id], <MixNodeRewarding>{
        cost_params: {},
        operator: "",
        delegates: "",
        total_unit_reward: "",
        unit_delegation: "",
        last_rewarded_epoch: 1,
        unique_delegations: 1,
    });

    tests("getOwnedMixnode", [mixnet, mixnodeowneraddress], <MixOwnershipResponse>{
        address: "",
        mixnode: {}
    });

    tests("getRewardParams", [mixnet], <string>{
    });

    tests("getRewardingStatus", [mixnet, mix_identity, rewardingIntervalNonce], <RewardingStatus>{
        Complete: {},
    });

    tests("getStakeSaturation", [mixnet, mix_id], <StakeSaturationResponse>{
        saturation: "",
        uncapped_saturation: "",
        as_at: 1n
    });

    tests("getStateParams", [mixnet], <ContractStateParams>{
        minimum_mixnode_pledge: "",
        minimum_gateway_pledge: "",
        mixnode_rewarded_set_size: 240,
        mixnode_active_set_size: 240,
    });

    tests("getUnbondedMixNodeInformation", [mixnet, mix_id], <UnbondedMixnodeResponse>{
        // Fix this
    });

    tests("ownsGateway", [mixnet, gatewayowneraddress], <GatewayOwnershipResponse>{
        address: gatewayowneraddress,
        gateway: {}
    });
});
