import expect from 'expect';
import {
  MixNodeRewarding,
  MixOwnershipResponse,
  PagedMixDelegationsResponse,
  PagedMixNodeBondResponse,
  PagedMixNodeDetailsResponse,
  UnbondedMixnodeResponse,
} from '@nymproject/types';
import { TestHelper } from './client';
import { mixnet, mixnodeowneraddress, mix_id } from './testData';

describe('Mixnode mock tests', () => {
  const testHelper = new TestHelper();

  it('get Mixnode Bonds', () => {
    const execute = testHelper.buildMethod('getMixNodeBonds', [mixnet], <PagedMixNodeBondResponse>{
      nodes: [],
      per_page: 25,
    });
    expect(execute).toBeTruthy();
  });

  it('get Mixnode Delegations Paged', () => {
    const execute = testHelper.buildMethod('getMixNodeDelegationsPaged', [mixnet, mix_id], <
      PagedMixDelegationsResponse
    >{
      delegations: [],
      per_page: 25,
    });
    expect(execute).toBeTruthy();
  });

  it('get Mixnodes Detailed', () => {
    const execute = testHelper.buildMethod('getMixNodesDetailed', [mixnet], <PagedMixNodeDetailsResponse>{
      nodes: [],
      per_page: 25,
    });
    expect(execute).toBeTruthy();
  });

  it('get Mixnode Rewarding Details', () => {
    const execute = testHelper.buildMethod('getMixnodeRewardingDetails', [mixnet, mix_id], <MixNodeRewarding>{
      cost_params: {},
      operator: '',
      delegates: '',
      total_unit_reward: '',
      unit_delegation: '',
      last_rewarded_epoch: 1,
      unique_delegations: 1,
    });
    expect(execute).toBeTruthy();
  });

  it('get Owned Mixnode', () => {
    const execute = testHelper.buildMethod('getOwnedMixnode', [mixnet, mixnodeowneraddress], <MixOwnershipResponse>{
      address: '',
      mixnode: {},
    });
    expect(execute).toBeTruthy();
  });

  it('get Unbonded Mixnode Information', () => {
    const execute = testHelper.buildMethod(
      'getUnbondedMixNodeInformation',
      [mixnet, mix_id],
      <UnbondedMixnodeResponse>{},
    );
    expect(execute).toBeTruthy();
  });
});
