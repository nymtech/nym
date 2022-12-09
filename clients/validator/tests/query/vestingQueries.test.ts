import ValidatorClient from '../../src';
import expect from 'expect';

const config = {
  rpcAddress: 'https://qwerty-validator.qa.nymte.ch',
  validatorAddress: 'https://qwerty-validator-api.qa.nymte.ch/api',
  prefix: 'n',
  mixnetContractAddress: 'n14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sjyvg3g',
  vestingContractAddress: 'n1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrq73f2nw',
  denom: 'nym',
};

describe('Vesting queries', () => {
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

  it('can query for contract version', async () => {
    const contract = await client.getVestingContractVersion();
    expect(contract).toBeTruthy();
  });
});
