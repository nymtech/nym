import expect from 'expect';
import { Delegation, PagedAllDelegationsResponse, PagedDelegatorDelegationsResponse } from '../../compiledTypes';
import { TestHelper } from './client';
import { mixnet, mixnodeowneraddress, mixId, mixIdentity } from './testData';

describe('Delegation mock tests', () => {
  const testHelper = new TestHelper();

  it('get Delegation Details', () => {
    const execute = testHelper.tests('getDelegationDetails', [mixnet, mixIdentity, mixnodeowneraddress], <Delegation>{
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
    const execute = testHelper.tests('getAllDelegationsPaged', [mixnet], <PagedAllDelegationsResponse>{
      delegations: [],
    });
    expect(execute).toBeTruthy();
  });

  it('get Delegator Delegations Paged', () => {
    const execute = testHelper.tests('getDelegatorDelegationsPaged', [mixnet, mixnodeowneraddress], <
      PagedDelegatorDelegationsResponse
    >{
      delegations: [],
    });
    expect(execute).toBeTruthy();
  });
});
