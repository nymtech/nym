import expect from 'expect';
import ValidatorClient from '../..';

const dotenv = require('dotenv');

dotenv.config();

// TODO: implement for QA with .env for mnemonics
describe('Simualtions', () => {
  let client: ValidatorClient;

  beforeEach(async () => {
    client = await ValidatorClient.connect(
      process.env.mnemonic || '',
      process.env.rpcAddress || '',
      process.env.validatorAddress || '',
      process.env.prefix || '',
      process.env.mixnetContractAddress || '',
      process.env.vestingContractAddress || '',
      process.env.denom || '',
    );
  });

  it('can simulate sending tokens', async () => {
    const res = await client.simulateSend({
      signingAddress: client.address,
      from: client.address,
      to: client.address,
      amount: [{ amount: '400000', denom: 'unym' }],
    });

    expect(typeof res).toBe('number');
  }).timeout(10000);
});
