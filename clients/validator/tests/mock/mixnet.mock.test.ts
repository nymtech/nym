import { INyxdQuery } from '../../src/query-client';
import {
  ContractStateParams,
  LayerDistribution,
  MixnetContractVersion,
  RewardingStatus,
  StakeSaturationResponse,
} from '../../compiledTypes';
import { Mock } from 'moq.ts';
import expect from 'expect';
import { TestHelper } from './client';

describe('Mixnet mock tests', () => {
  let mixnet = 'n14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sjyvg3g';
  let mix_id = 436207616;
  let mix_identity = '26';
  let rewardingIntervalNonce = 1;

  let client: Mock<INyxdQuery>;
  let testHelper = new TestHelper();

  beforeEach(() => {
    client = new Mock<INyxdQuery>();
  });

  it('get Layer Distribution', async () => {
    let execute = testHelper.tests(client, 'getLayerDistribution', [mixnet], <LayerDistribution>{
      gateways: 10,
      layer1: 2,
      layer2: 2,
      layer3: 5,
    });
    expect(execute).toBeTruthy();
  });

  it('get Reward Params', async () => {
    let execute = testHelper.tests(client, 'getRewardParams', [mixnet], <string>{});
    expect(execute).toBeTruthy();
  });

  it('get Rewarding Status', async () => {
    let execute = testHelper.tests(client, 'getRewardingStatus', [mixnet, mix_identity, rewardingIntervalNonce], <
      RewardingStatus
    >{
      Complete: {},
    });
    expect(execute).toBeTruthy();
  });

  it('get Stake Saturation', async () => {
    let execute = testHelper.tests(client, 'getStakeSaturation', [mixnet, mix_id], <StakeSaturationResponse>{
      saturation: '',
      uncapped_saturation: '',
      as_at: 1n,
    });
    expect(execute).toBeTruthy();
  });

  it('get State Params', async () => {
    let execute = testHelper.tests(client, 'getStateParams', [mixnet], <ContractStateParams>{
      minimum_mixnode_pledge: '',
      minimum_gateway_pledge: '',
      mixnode_rewarded_set_size: 240,
      mixnode_active_set_size: 240,
    });
    expect(execute).toBeTruthy();
  });

  it('get Contract Version', async () => {
    let execute = testHelper.tests(client, 'getContractVersion', [mixnet], <MixnetContractVersion>{
      build_timestamp: 'test',
      commit_branch: 'test',
      build_version: 'test',
      rustc_version: 'test',
      commit_sha: 'test',
      commit_timestamp: 'test',
    });
    expect(execute).toBeTruthy();
  });
});
