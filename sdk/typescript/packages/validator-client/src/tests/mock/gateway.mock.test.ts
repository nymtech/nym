import expect from 'expect';
import { GatewayOwnershipResponse, PagedGatewayResponse } from '../../types/shared-types';
import { TestHelper } from './client';
import { gatewayowneraddress, mixnet } from './testData';

describe('Gateway mock tests', () => {
  const testHelper = new TestHelper();

  it('get Gateways Paged', () => {
    const execute = testHelper.buildMethod('getGatewaysPaged', [mixnet], <PagedGatewayResponse>{
      nodes: [],
      per_page: 25,
    });
    expect(execute).toBeTruthy();
  });

  it('owns Gateway', () => {
    const execute = testHelper.buildMethod('ownsGateway', [mixnet, gatewayowneraddress], <GatewayOwnershipResponse>{
      address: gatewayowneraddress,
      gateway: {},
    });
    expect(execute).toBeTruthy();
  });
});
