import ValidatorClient from '../../dist';
import expect from 'expect';

const config = {
  rpcAddress: 'https://qwerty-validator.qa.nymte.ch',
  validatorAddress: 'https://qwerty-validator-api.qa.nymte.ch/api',
  prefix: 'n',
  mixnetContractAddress: 'n14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sjyvg3g',
  vestingContractAddress: 'n1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrq73f2nw',
  denom: 'nym',
};

describe('Network queries', async () => {
  let client: ValidatorClient;

  beforeEach(async () => {
    client = await ValidatorClient.connectForQuery(
      config.rpcAddress,
      config.validatorAddress,
      config.prefix,
      config.mixnetContractAddress,
      config.vestingContractAddress,
      config.denom,
    );
  });

  it('can query for an account balance', async () => {
    const balance = await client.getBalance('n1ptg680vnmef2cd8l0s9uyc4f0hgf3x8sed6w77');
    expect(Number.parseFloat(balance.amount)).toBeGreaterThan(0);
  }).timeout(5000);

  it('can query for stake saturation', async () => {
    const stakeSaturation = await client.getStakeSaturation(8);
    expect(stakeSaturation).toBeTruthy();
  });

  it('can query for unbonded mixnodes', async () => {
    const unbondedNodes = await client.getUnbondedMixNodes();
    expect(unbondedNodes).toBeTruthy();
  });

  it('can query for unbonded mixnode information', async () => {
    const unbondedMixnodeInfo = await client.getUnbondedMixNodeInformation(1);
    expect(unbondedMixnodeInfo).toBeTruthy();
  });

  it('can query for mixnode rewarding details', async () => {
    const rewardingDetails = await client.getMixnodeRewardingDetails(1);
    expect(rewardingDetails).toBeTruthy();
  });

  it('can query for owned mixnode', async () => {
    const ownedMixnode = await client.getOwnedMixnode('n1ptg680vnmef2cd8l0s9uyc4f0hgf3x8sed6w77');
    expect(ownedMixnode).toBeTruthy();
  });

  it('can query for all mixnode bonds', async () => {
    const mixnodeBonds = await client.getMixNodeBonds();
    expect(mixnodeBonds).toBeTruthy();
  });

  it('can query for all mixnode details', async () => {
    const mixnodeBonds = await client.getMixNodesDetailed();
    expect(mixnodeBonds).toBeTruthy();
  });
});
