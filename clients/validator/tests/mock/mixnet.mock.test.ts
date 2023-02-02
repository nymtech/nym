import expect from 'expect';
import {
  ContractStateParams,
  LayerDistribution,
  MixnetContractVersion,
  StakeSaturationResponse,
} from '../../compiledTypes';
import { TestHelper } from './client';
import { mixnet, mix_id } from './testData';
import { RewardingParams } from '../../compiledTypes/types/global';

describe('Mixnet mock tests', () => {
  const testHelper = new TestHelper();

  it('get Layer Distribution', () => {
    const execute = testHelper.buildMethod('getLayerDistribution', [mixnet], <LayerDistribution>{
      layer1: 2,
      layer2: 2,
      layer3: 5,
    });
    expect(execute).toBeTruthy();
  });

  it('get Reward Params', () => {
    const execute = testHelper.buildMethod('getRewardParams', [mixnet], <RewardingParams>{
      interval: {},
      rewarded_set_size: 0,
      active_set_size: 0,
    });
    expect(execute).toBeTruthy();
  });

  it('get Stake Saturation', () => {
    const execute = testHelper.buildMethod('getStakeSaturation', [mixnet, mix_id], <StakeSaturationResponse>{
      mix_id: 0,
      current_saturation: '',
      uncapped_saturation: '',
    });
    expect(execute).toBeTruthy();
  });

  it('get State Params', () => {
    const execute = testHelper.buildMethod('getStateParams', [mixnet], <ContractStateParams>{
      minimum_mixnode_pledge: '',
      minimum_gateway_pledge: '',
      mixnode_rewarded_set_size: 240,
      mixnode_active_set_size: 240,
    });
    expect(execute).toBeTruthy();
  });

  it('get Contract Version', () => {
    const execute = testHelper.buildMethod('getContractVersion', [mixnet], <MixnetContractVersion>{
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
