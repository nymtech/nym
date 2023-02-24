import expect from 'expect';
import ValidatorClient from '../../';

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

  it.skip('can delegate tokens', async () => {
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

  it.skip('Can unbond a mixnode', async () => {
    const res = await client.unbondMixNode();
    expect(res.transactionHash).toBeDefined();
  }).timeout(10000);

  it.skip('Can bond a mixnode', async () => {
    const res = await client.bondMixNode(
      {
        identity_key: '3P6pAcF2p3pYMqWdpHqhbavu3ifyaBs5Qw5UmmCGwimx',
        sphinx_key: 'GQMQKwUThaggatA6oZteSWTsCQoUfmLtamJ7o9YkP9aE',
        host: '109.74.195.67',
        mix_port: 1789,
        verloc_port: 1790,
        http_api_port: 8000,
        version: '1.1.4',
      },
      '3rXWCQBUj5JQB3UBUkZcXhCk9Zh3cjduMF8aFHPTG7KTkkhZzDJTNmE2p2Ph1g6iQW5vRGTpQzjXF33WDwvhzHk6',
      { profit_margin_percent: '0.1', interval_operating_cost: { amount: '40', denom: 'nym' } },
      { amount: '100_000_000', denom: 'unym' },
      { gas: '1000000', amount: [{ amount: '100000', denom: 'unym' }] },
    );
    expect(res.transactionHash).toBeDefined();
  }).timeout(10000);

  it.skip('can unbond a gateway', async () => {
    const res = await client.unbondGateway();
    expect(res.transactionHash).toBeDefined();
  });

  it.skip('can bond a gateway', async () => {
    const res = await client.bondGateway(
      {
        identity_key: '36vfvEyBzo5cWEFbnP7fqgY39kFw9PQhvwzbispeNaxL',
        sphinx_key: 'G65Fwc2JNAotuHQFqmDKhQNQL5rn3r9pupUdmxMygNUZ',
        host: '151.236.220.82',
        version: '1.1.4',
        mix_port: 1789,
        clients_port: 9000,
        location: 'Cuba',
      },
      '3ipSJksWHehZm1YfuH5Ahtg7b22NnrP9hEs6iEDXfUS5uiUhpWmCjGR3b3NDHuxeGjpZYJNYJ52D8WCPK5ZR7Szj',
      { amount: '100_000_000', denom: 'unym' },
    );
    expect(res.transactionHash).toBeDefined();
  });

  it.skip('can update contract state params', async () => {
    const res = await client.updateContractStateParams({
      minimum_mixnode_pledge: '',
      minimum_gateway_pledge: '',
      mixnode_rewarded_set_size: 2,
      mixnode_active_set_size: 2,
    });
    expect(res.transactionHash).toBeDefined();
  });
});
