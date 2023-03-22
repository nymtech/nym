import expect from 'expect';
import { Delegation } from '@nymproject/types';
import { PagedAllDelegationsResponse, PagedDelegatorDelegationsResponse } from '../../types/shared-types';
import { TestHelper } from './client';
import { mixnet, mixnodeowneraddress, mix_id } from './testData';

describe('Delegation mock tests', () => {
  const testHelper = new TestHelper();

  it('get Delegation Details', () => {
    const execute = testHelper.buildMethod('getDelegationDetails', [mixnet, mix_id, mixnodeowneraddress], <Delegation>{
      owner: mixnodeowneraddress,
      mix_id: 0,
      cumulative_reward_ratio: '',
      amount: {
        denom: 'nym',
        amount: '10',
      },
      height: 1314134144132n,
      proxy: 'null',
    });
    expect(execute).toBeTruthy();
  });

  it('get All Delegations Paged', () => {
    const execute = testHelper.buildMethod('getAllDelegationsPaged', [mixnet], <PagedAllDelegationsResponse>{
      delegations: [],
    });
    expect(execute).toBeTruthy();
  });

  it('get Delegator Delegations Paged', () => {
    const execute = testHelper.buildMethod('getDelegatorDelegationsPaged', [mixnet, mixnodeowneraddress], <
      PagedDelegatorDelegationsResponse
    >{
      delegations: [],
    });
    expect(execute).toBeTruthy();
  });
});
