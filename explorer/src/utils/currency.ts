import { printableCoin } from '@nymproject/nym-validator-client';
import Big from 'big.js';
import { DecCoin, isValidRawCoin } from '@nymproject/types';

const DENOM = process.env.CURRENCY_DENOM || 'unym';
const DENOM_STAKING = process.env.CURRENCY_STAKING_DENOM || 'unyx';

export const toDisplay = (val: string | number | Big, dp = 4) => {
  let displayValue;
  try {
    displayValue = Big(val).toFixed(dp);
  } catch (e: any) {
    console.warn(`${displayValue} not a valid decimal number: ${e}`);
  }
  return displayValue;
};

export const currencyToString = ({ amount, dp, denom = DENOM }: { amount: string; dp?: number; denom?: string }) => {
  if (!dp) {
    printableCoin({
      amount,
      denom,
    });
  }

  const [printableAmount, printableDenom] = printableCoin({
    amount,
    denom,
  }).split(/\s+/);

  return `${toDisplay(printableAmount, dp)} ${printableDenom}`;
};

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

export const validateAmount = async (
  majorAmountAsString: DecCoin['amount'],
  minimumAmountAsString: DecCoin['amount'],
): Promise<boolean> => {
  // tests basic coin value requirements, like no more than 6 decimal places, value lower than total supply, etc
  if (!Number(majorAmountAsString)) {
    return false;
  }

  if (!isValidRawCoin(majorAmountAsString)) {
    return false;
  }

  const majorValueFloat = parseInt(majorAmountAsString, Number(10));

  return majorValueFloat >= parseInt(minimumAmountAsString, Number(10));
};

/**
 * Takes a DecCoin and prettify its amount to a representation
 * with fixed decimal places.
 *
 * @param coin - a DecCoin
 * @param dp - number of decimal places to apply to amount (4 by default ie. 0.0000)
 * @returns A DecCoin with prettified amount
 */
export const decCoinToDisplay = (coin: DecCoin, dp = 4) => {
  const displayCoin = { ...coin };
  try {
    displayCoin.amount = Big(coin.amount).toFixed(dp);
  } catch (e: any) {
    console.warn(`${coin.amount} not a valid decimal number: ${e}`);
  }
  return displayCoin;
};
