import ValidatorClient from '../../dist';
import expect from 'expect';
const dotenv = require('dotenv');

dotenv.config();

// TODO: implement for QA with .env for mnemonics
describe('Mixnet actions', () => {
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

  it('can send tokens', async () => {
    const res = await client.send(client.address, [{ amount: '10000000', denom: 'unym' }]);
    expect(res.transactionHash).toBeDefined();
  }).timeout(10000);

  it('can delegate tokens', async () => {
    const [_, secondMixnode] = await client.getActiveMixnodes();

    if (secondMixnode) {
      const res = await client.delegateToMixNode(
        secondMixnode.bond_information.mix_id,
        {
          amount: '15000000',
          denom: 'unym',
        },
        { gas: '1000000', amount: [{ amount: '100000', denom: 'unym' }] },
      );
      expect(res.transactionHash).toBeDefined();
    }
  }).timeout(10000);

  // Need to provide a mix id that can be undelegated from
  it.skip('can undelegate from a mixnode', async () => {
    const mixId = 8;
    const res = await client.undelegateFromMixNode(mixId);
    expect(res.transactionHash).toBeDefined();
  });
});
