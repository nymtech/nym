import expect from 'expect';
import ValidatorClient from '../../index';
import { amountDemon, Delegations, DelegatorTimes, Node, OriginalVestingDetails, VestingAccountDetails, vestingAccountsPaged, VestingCoinAccounts, VestingPeriod } from '../expectedResponses';

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

  const vesting_account_address = 'n14juvj7llvx8eppypnqj6xlrgwss9wfrcuy0nkv';
  const mixnodeowner = 'n1z93z44vf8ssvdhujjvxcj4rd5e3lz0l60wdk70';
  const gatewayowner = 'n1un9cuvw9e3xqratmde4j55ucksev0dkeruq800';
  const mix_id = 79;

  it('can query for contract version', async () => {
    const contract = await client.getVestingContractVersion();
    expect(contract).toBeTruthy();
  });

  // TODO see if we can use AccountEntry type here instead
  it('can get all accounts paged', async () => {
    const accounts = await client.getVestingAccountsPaged();
    expect(Object.keys(accounts)).toEqual(Object.keys(vestingAccountsPaged));
    expect(accounts).toBeTruthy();
  });

  it('can get coins for all accounts paged', async () => {
    const accounts = await client.getVestingAmountsAccountsPaged();
    expect(Object.keys(accounts)).toEqual(Object.keys(VestingCoinAccounts));
    expect(accounts).toBeTruthy();
  });

  it('can get locked tokens for an account', async () => {
    const locked = await client.getLockedTokens(vesting_account_address);
    expect(Object.keys(locked)).toEqual(Object.keys(amountDemon));
    expect(locked).toBeTruthy();
  });

  it('can get spendable tokens for an account', async () => {
    const spendable = await client.getSpendableTokens(vesting_account_address);
    expect(Object.keys(spendable)).toEqual(Object.keys(amountDemon));
    expect(spendable).toBeTruthy();
  });

  it('can get vested tokens for an account', async () => {
    const vested = await client.getVestedTokens(vesting_account_address);
    expect(Object.keys(vested)).toEqual(Object.keys(amountDemon));
    expect(vested).toBeTruthy();
  });

  it('can get vesting tokens for an account', async () => {
    const vesting = await client.getVestingTokens(vesting_account_address);
    expect(Object.keys(vesting)).toEqual(Object.keys(amountDemon));
    expect(vesting).toBeTruthy();
  });

  it('can get spendable vested tokens for an account', async () => {
    const spendable = await client.getSpendableVestedTokens(vesting_account_address);
    expect(Object.keys(spendable)).toEqual(Object.keys(amountDemon));
    expect(spendable).toBeTruthy();
  });

  it('can get spendable rewards for an account', async () => {
    const rewards = await client.getSpendableRewards(vesting_account_address);
    expect(Object.keys(rewards)).toEqual(Object.keys(amountDemon));
    expect(rewards).toBeTruthy();
  });

  it('can get delegated coins', async () => {
    const delegated = await client.getDelegatedCoins(vesting_account_address);
    expect(Object.keys(delegated)).toEqual(Object.keys(amountDemon));
    expect(delegated).toBeTruthy();
  });

  it('can get pledged coins', async () => {
    const pledged = await client.getPledgedCoins(vesting_account_address);
    expect(Object.keys(pledged)).toEqual(Object.keys(amountDemon));
    expect(pledged).toBeTruthy();
  });

  it('can get staked coins', async () => {
    const staked = await client.getStakedCoins(vesting_account_address);
    expect(Object.keys(staked)).toEqual(Object.keys(amountDemon));
    expect(staked).toBeTruthy();
  });

  it('can get withdrawn coins', async () => {
    const withdrawn = await client.getWithdrawnCoins(vesting_account_address);
    expect(Object.keys(withdrawn)).toEqual(Object.keys(amountDemon));
    expect(withdrawn).toBeTruthy();
  });

  it('can get start time of an account', async () => {
    const time = await client.getStartTime(vesting_account_address);
    expect(typeof time).toBe("string");
    expect(time).toBeTruthy();
  });

  it('can get end time of an account', async () => {
    const time = await client.getEndTime(vesting_account_address);
    expect(typeof time).toBe("string");
    expect(time).toBeTruthy();
  });

  it('can get account original vesting details', async () => {
    const original = await client.getOriginalVestingDetails(vesting_account_address);
    expect(Object.keys(original)).toEqual(Object.keys(OriginalVestingDetails));
    expect(original).toBeTruthy();
  });

  it('can get historic vesting staking rewards', async () => {
    const rewards = await client.getHistoricStakingRewards(vesting_account_address);
    expect(Object.keys(rewards)).toEqual(Object.keys(amountDemon));
    expect(rewards).toBeTruthy();
  });

  // TODO see if we can use "VestingAccountInfo" type here instead
  it('can get account details', async () => {
    const account = await client.getAccountDetails(vesting_account_address);
    expect(Object.keys(account)).toEqual(Object.keys(VestingAccountDetails));
    expect(account).toBeTruthy();
  });

  // TODO add option for if account has no mixnode and expected is null 
  it('can get mixnode', async () => {
    const mixnode = await client.getMixnode(mixnodeowner);
    expect(Object.keys(mixnode)).toEqual(Object.keys(Node));
    expect(mixnode).toBeTruthy();
  });

  // TODO add option for if account has no gateway and expected is null 
  it.skip('can get gateway', async () => {
    const gateway = await client.getGateway(gatewayowner);
    expect(Object.keys(gateway)).toEqual(Object.keys(Node));
    expect(gateway).toBeTruthy();
  });

  it('can get delegations times', async () => {
    const delegation = await client.getDelegationTimes(mix_id, mixnodeowner);
    expect(Object.keys(delegation)).toEqual(Object.keys(DelegatorTimes));
    expect(delegation).toBeTruthy();
  });

  it('can get all delegations', async () => {
    const delegation = await client.getAllDelegations();
    expect(Object.keys(delegation)).toEqual(Object.keys(Delegations));
    expect(delegation).toBeTruthy();
  });

  it('can get current vesting period', async () => {
    const period = await client.getCurrentVestingPeriod(gatewayowner);
    expect(period).toEqual(expect.anything() as unknown as VestingPeriod);
    expect(period).toBeTruthy();
  });
});
