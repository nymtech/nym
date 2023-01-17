import expect from 'expect';
import ValidatorClient from '../../src';
const dotenv = require('dotenv');

dotenv.config();

describe('Vesting queries', () => {
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

  it('can query for contract version', async () => {
    const contract = await client.getVestingContractVersion();
    expect(contract).toBeTruthy();
  });

  it('can get the balance on the account', () => {});
});
