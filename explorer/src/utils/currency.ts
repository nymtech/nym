import { printableCoin } from '@nymproject/nym-validator-client';

const DENOM = process.env.CURRENCY_DENOM || 'unym';
const DENOM_STAKING = process.env.CURRENCY_STAKING_DENOM || 'unyx';

export const currencyToString = (amount: string, denom: string = DENOM) =>
  printableCoin({
    amount,
    denom,
  });

export const stakingCurrencyToString = (amount: string, denom: string = DENOM_STAKING) =>
  printableCoin({
    amount,
    denom,
  });
