import expect from 'expect';
import {
  ContractStateParams,
  LayerDistribution,
  MixnetContractVersion,
  RewardingStatus,
  StakeSaturationResponse,
} from '../../compiledTypes';
import { TestHelper } from './client';
import { mixId, mixnet, mixIdentity, rewardingIntervalNonce } from './testData';
import { RewardingParams } from '../../compiledTypes/types/global';

describe('Mixnet mock tests', () => {
  const testHelper = new TestHelper();

  it('get Layer Distribution', () => {
    const execute = testHelper.tests('getLayerDistribution', [mixnet], <LayerDistribution>{
      gateways: 10,
      layer1: 2,
      layer2: 2,
      layer3: 5,
    });
    expect(execute).toBeTruthy();
  });

  it('get Reward Params', () => {
    const execute = testHelper.tests('getRewardParams', [mixnet], <RewardingParams>{
      interval: {},
      rewarded_set_size: 0,
      active_set_size: 0,
    });
    expect(execute).toBeTruthy();
  });

  it('get Rewarding Status', () => {
    const execute = testHelper.tests('getRewardingStatus', [mixnet, mixIdentity, rewardingIntervalNonce], <
      RewardingStatus
    >{
      Complete: {},
    });
    expect(execute).toBeTruthy();
  });

  it('get Stake Saturation', () => {
    const execute = testHelper.tests('getStakeSaturation', [mixnet, mixId], <StakeSaturationResponse>{
      mix_id: 0,
      current_saturation: '',
      uncapped_saturation: '',
    });
    expect(execute).toBeTruthy();
  });

  it('get State Params', () => {
    const execute = testHelper.tests('getStateParams', [mixnet], <ContractStateParams>{
      minimum_mixnode_pledge: '',
      minimum_gateway_pledge: '',
      mixnode_rewarded_set_size: 240,
      mixnode_active_set_size: 240,
    });
    expect(execute).toBeTruthy();
  });

  it('get Contract Version', () => {
    const execute = testHelper.tests('getContractVersion', [mixnet], <MixnetContractVersion>{
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
