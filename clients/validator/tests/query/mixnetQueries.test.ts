import ValidatorClient from '../../src';
import expect from 'expect';
const dotenv = require('dotenv');
import { allunbondednodes, contract, delegation, gateway, mixnode, mixnodebond, ownedNode, rewardingnode, saturation, unbondednode } from '../../types/expectedResponses';

dotenv.config();

describe('Mixnet queries', () => {
  let client: ValidatorClient;

  beforeEach(async () => {
    client = await ValidatorClient.connectForQuery(
      process.env.rpcAddress || '',
      process.env.validatorAddress || '',
      process.env.prefix || '',
      process.env.mixnetContractAddress || '',
      process.env.vestingContractAddress || '',
      process.env.denom || '',
    );
  });

  it('can query for an account balance', async () => {
    const balance = await client.getBalance('n1ptg680vnmef2cd8l0s9uyc4f0hgf3x8sed6w77');
    expect(Number.parseFloat(balance.amount)).toBeGreaterThan(0);
  });

  it('can query for stake saturation', async () => {
    const stakeSaturation = await client.getStakeSaturation(8);
    expect(stakeSaturation).toBeTruthy();
    expect(stakeSaturation?.current_saturation).toBeTruthy();
  });

  it('can query for contract version', async () => {
    const contract = await client.getMixnetContractVersion();
    expect(contract).toBeTruthy();
  });

  // TODO Needs fixing
  it.skip('can query for mixnet contract settings', async () => {
    const settings = await client.getMixnetContractSettings();
    console.log(settings);
    expect(Object.keys(settings)).toEqual(Object.keys(contract));
    expect(settings).toBeTruthy;
  });

  it('can query for unbonded mixnodes', async () => {
    const unbondedNodes = await client.getUnbondedMixNodes();
    for(let i = 0; i < unbondedNodes.length; i++){
      expect(Object.keys(unbondedNodes[0])).toEqual(Object.keys(allunbondednodes));
      expect(unbondedNodes).toBeTruthy();
  }
  });

  it('can query for unbonded mixnode information', async () => {
    const unbondedMixnodeInfo = await client.getUnbondedMixNodeInformation(1);
    expect(Object.keys(unbondedMixnodeInfo)).toEqual(Object.keys(unbondednode));
    expect(unbondedMixnodeInfo).toBeTruthy();
  });

  it('can query for mixnode rewarding details', async () => {
    const rewardingDetails = await client.getMixnodeRewardingDetails(1);
    expect(Object.keys(rewardingDetails)).toEqual(Object.keys(rewardingnode));
    expect(rewardingDetails).toBeTruthy();
  });

  it('can query for owned mixnode', async () => {
    const ownedMixnode = await client.getOwnedMixnode('n1ptg680vnmef2cd8l0s9uyc4f0hgf3x8sed6w77');
    expect(Object.keys(ownedMixnode)).toEqual(Object.keys(ownedNode));
    expect(ownedMixnode).toBeTruthy();
  });

  it('can query for all mixnode bonds', async () => {
    const mixnodeBonds = await client.getMixNodeBonds();
    expect(Object.keys(mixnodeBonds[0])).toEqual(Object.keys(mixnodebond));
    expect(mixnodeBonds).toBeTruthy();
    expect(Array.isArray(mixnodeBonds)).toBeTruthy();
  });

  it('can query for all mixnode details', async () => {
    const mixnodeDetails = await client.getMixNodesDetailed();
    expect(Object.keys(mixnodeDetails[0])).toEqual(Object.keys(mixnode));
    expect(mixnodeDetails).toBeTruthy();
    expect(Array.isArray(mixnodeDetails)).toBeTruthy();
  });

  it('can query for account delegations', async () => {
    const delegations = await client.getAllNyxdDelegatorDelegations('n1fzv4jc7fanl9s0qj02ge2ezk3kts545kjtek47');
    expect(Object.keys(delegations[0])).toEqual(Object.keys(delegation));
    expect(delegations).toBeTruthy;
    expect(Array.isArray(delegations)).toBeTruthy();
  });

  it('can query for all delegations', async () => {
    const allDelegations = await client.getAllNyxdDelegations();
    expect(Object.keys(allDelegations[0])).toEqual(Object.keys(delegation));
    expect(allDelegations).toBeTruthy;
    expect(Array.isArray(allDelegations)).toBeTruthy();
  });

  it('can query for all delegations on a node', async () => {
    const mixnodeDelegations = await client.getAllNyxdSingleMixnodeDelegations(1);
    expect(Object.keys(mixnodeDelegations[0])).toEqual(Object.keys(delegation));
    expect(mixnodeDelegations).toBeTruthy;
  });

  it('can query for all gateways', async () => {
    const gateways = await client.getAllNyxdGateways();
    expect(Object.keys(gateways[0])).toEqual(Object.keys(gateway));
    expect(gateways).toBeTruthy();
    expect(Array.isArray(gateways)).toBeTruthy();
  }).timeout(10000);

  it('can query for all active mixnodes', async () => {
    const activeNodes = await client.getActiveMixnodes();
    expect(Object.keys(activeNodes[0])).toEqual(Object.keys(mixnode));
    expect(activeNodes).toBeTruthy();
    expect(Array.isArray(activeNodes)).toBeTruthy();
  });

  it('can query for reward pool', async () => {
    const rewardPool = await client.getRewardParams();
    // TODO add velidation here
    expect(rewardPool).toBeTruthy();
  });

  it('can query for rewarded mixnodes', async () => {
    const rewardNodes = await client.getRewardedMixnodes();
    expect(Object.keys(rewardNodes[0])).toEqual(Object.keys(mixnode));
    expect(rewardNodes).toBeTruthy();
  });

  it('can query for stake saturation', async () => {
    const stakeSaturation = await client.getStakeSaturation(7);
    expect(Object.keys(stakeSaturation)).toEqual(Object.keys(saturation));
    expect(stakeSaturation).toBeTruthy();
  });
});
