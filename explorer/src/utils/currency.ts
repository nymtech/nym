import { printableCoin } from '@nymproject/nym-validator-client';
import Big from 'big.js';

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

/**
 * Converts a decimal number to a pretty representation
 * with fixed decimal places.
 *
 * @param val - a decimal number of string form
 * @param dp - number of decimal places (4 by default ie. 0.0000)
 * @returns A prettyfied decimal number
 */
export const toDisplay = (val: string | number | Big, dp = 4) => {
  let displayValue;
  try {
    displayValue = Big(val).toFixed(dp);
  } catch (e: any) {
    console.warn(`${displayValue} not a valid decimal number: ${e}`);
  }
  return displayValue;
};

/**
 * Converts a decimal number of μNYM (micro NYM) to NYM.
 *
 * @param unym - a decimal number of μNYM
 * @param dp - number of decimal places (4 by default ie. 0.0000)
 * @returns The corresponding decimal number in NYM
 */
export const unymToNym = (unym: string | number | Big, dp = 4) => {
  let nym;
  try {
    nym = Big(unym).div(1_000_000).toFixed(dp);
  } catch (e: any) {
    console.warn(`${unym} not a valid decimal number: ${e}`);
  }
  return nym;
};
