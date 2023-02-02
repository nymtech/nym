import expect from 'expect';
import {
  MixNodeRewarding,
  MixOwnershipResponse,
  PagedMixDelegationsResponse,
  PagedMixNodeBondResponse,
  PagedMixNodeDetailsResponse,
  UnbondedMixnodeResponse,
} from '../../compiledTypes';
import { TestHelper } from './client';
import { mixnet, mixnodeowneraddress, mixId, mixIdentity } from './testData';

describe('Mixnode mock tests', () => {
  const testHelper = new TestHelper();

  it('get Mixnode Bonds', () => {
    const execute = testHelper.tests('getMixNodeBonds', [mixnet], <PagedMixNodeBondResponse>{
      nodes: [],
      per_page: 25,
    });
    expect(execute).toBeTruthy();
  });

  it('get Mixnode Delegations Paged', () => {
    const execute = testHelper.tests('getMixNodeDelegationsPaged', [mixnet, mixIdentity], <PagedMixDelegationsResponse>{
      delegations: [],
      per_page: 25,
    });
    expect(execute).toBeTruthy();
  });

  it('get Mixnodes Detailed', () => {
    const execute = testHelper.tests('getMixNodesDetailed', [mixnet], <PagedMixNodeDetailsResponse>{
      nodes: [],
      per_page: 25,
    });
    expect(execute).toBeTruthy();
  });

  it('get Mixnode Rewarding Details', () => {
    const execute = testHelper.tests('getMixnodeRewardingDetails', [mixnet, mixId], <MixNodeRewarding>{
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
    const execute = testHelper.tests('getOwnedMixnode', [mixnet, mixnodeowneraddress], <MixOwnershipResponse>{
      address: '',
      mixnode: {},
    });
    expect(execute).toBeTruthy();
  });

  it('get Unbonded Mixnode Information', () => {
    const execute = testHelper.tests('getUnbondedMixNodeInformation', [mixnet, mixId], <UnbondedMixnodeResponse>{});
    expect(execute).toBeTruthy();
  });
});
