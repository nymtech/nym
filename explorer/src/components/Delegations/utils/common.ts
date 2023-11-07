import { appWindow } from '@tauri-apps/api/window';
import bs58 from 'bs58';
import Big from 'big.js';
import { valid } from 'semver';
import { add, format, fromUnixTime } from 'date-fns';
import { DecCoin, isValidRawCoin, MixNodeCostParams } from '@nymproject/types';
import { getCurrentInterval, getDefaultMixnodeCostParams, getLockedCoins, getSpendableCoins } from '../requests';
import { Console } from './console';

export type TPoolOption = 'balance' | 'locked';

export const uNYMtoNYM = (unym: string, rounding = 6) => {
  const nym = Big(unym).div(1000000).toFixed(rounding);

  return {
    asString: () => nym,
    asNumber: () => Number(nym),
  };
};

// export const checkHasEnoughFunds = async (balance: string, allocationValue: string): Promise<boolean> => {
//   try {
//     // const walletValue = await userBalance();

//     const remainingBalance = +balance - +allocationValue;
//     return remainingBalance >= 0;
//   } catch (e) {
//     Console.log(e as string);
//     return false;
//   }
// };

export const checkHasEnoughLockedTokens = async (allocationValue: string) => {
  try {
    const lockedTokens = await getLockedCoins();
    const spendableTokens = await getSpendableCoins();
    const remainingBalance = +lockedTokens.amount + +spendableTokens.amount - +allocationValue;
    return remainingBalance >= 0;
  } catch (e) {
    Console.error(e as string);
  }
  return false;
};

export const checkTokenBalance = (tokenPool: TPoolOption, amount: string, balance: string) => {
  let hasEnoughFunds = false;
  // if (tokenPool === 'locked') {
  //   hasEnoughFunds = await checkHasEnoughLockedTokens(amount);
  // }

  if (tokenPool === 'balance') {
    const remainingBalance = +balance - +amount;
    hasEnoughFunds = remainingBalance >= 0;
  }

  return hasEnoughFunds;
};

export const validateKey = (key: string, bytesLength: number): boolean => {
  // it must be a valid base58 key
  try {
    const bytes = bs58.decode(key);
    // of length 32
    return bytes.length === bytesLength;
  } catch (e) {
    Console.error(e as string);
    return false;
  }
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

  // this conversion seems really iffy but I'm not sure how to better approach it
};

export const isValidHostname = (value: string) => {
  // regex for ipv4 and ipv6 and hhostname- source http://jsfiddle.net/DanielD/8S4nq/
  const hostnameRegex =
    /((^((([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5])\.){3}([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5]))\s*$)|(^\s*((([0-9A-Fa-f]{1,4}:){7}([0-9A-Fa-f]{1,4}|:))|(([0-9A-Fa-f]{1,4}:){6}(:[0-9A-Fa-f]{1,4}|((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3})|:))|(([0-9A-Fa-f]{1,4}:){5}(((:[0-9A-Fa-f]{1,4}){1,2})|:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3})|:))|(([0-9A-Fa-f]{1,4}:){4}(((:[0-9A-Fa-f]{1,4}){1,3})|((:[0-9A-Fa-f]{1,4})?:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9A-Fa-f]{1,4}:){3}(((:[0-9A-Fa-f]{1,4}){1,4})|((:[0-9A-Fa-f]{1,4}){0,2}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9A-Fa-f]{1,4}:){2}(((:[0-9A-Fa-f]{1,4}){1,5})|((:[0-9A-Fa-f]{1,4}){0,3}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9A-Fa-f]{1,4}:){1}(((:[0-9A-Fa-f]{1,4}){1,6})|((:[0-9A-Fa-f]{1,4}){0,4}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(:(((:[0-9A-Fa-f]{1,4}){1,7})|((:[0-9A-Fa-f]{1,4}){0,5}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:)))(%.+)?\s*$))|(^\s*((?=.{1,255}$)(?=.*[A-Za-z].*)[0-9A-Za-z](?:(?:[0-9A-Za-z]|\b-){0,61}[0-9A-Za-z])?(?:\.[0-9A-Za-z](?:(?:[0-9A-Za-z]|\b-){0,61}[0-9A-Za-z])?)*)\s*$)/;

  return hostnameRegex.test(value);
};

export const validateVersion = (version: string): boolean => {
  try {
    return valid(version) !== null;
  } catch (e) {
    return false;
  }
};

export const validateLocation = (location: string): boolean => {
  const locationRegex = /^[a-z]+$/i;
  return locationRegex.test(location);
};

export const validateRawPort = (rawPort: number): boolean => !Number.isNaN(rawPort) && rawPort >= 1 && rawPort <= 65535;

export const truncate = (text: string, trim: number) => `${text.substring(0, trim)}...`;

export const isGreaterThan = (a: number, b: number) => a > b;

export const isLessThan = (a: number, b: number) => a < b;

export const randomNumberBetween = (min: number, max: number) => {
  const minCeil = Math.ceil(min);
  const maxFloor = Math.floor(max);
  return Math.floor(Math.random() * (maxFloor - minCeil + 1) + minCeil);
};

export const splice = (size: number, address?: string): string => {
  if (address) {
    return `${address.slice(0, size)}...${address.slice(-size)}`;
  }
  return '';
};

export const maximizeWindow = async () => {
  await appWindow.maximize();
};

export function removeObjectDuplicates<T extends object, K extends keyof T>(arr: T[], id: K) {
  return arr.filter((v, i, a) => a.findIndex((v2) => v2[id] === v[id]) === i);
}

export const isDecimal = (value: number) => value - Math.floor(value) !== 0;

export const attachDefaultOperatingCost = async (profitMarginPercent: string): Promise<MixNodeCostParams> =>
  getDefaultMixnodeCostParams(profitMarginPercent);

/**
 * Converts a stringified percentage integer (0-100) to a stringified float (0.0-1.0).
 *
 * @param value - the percentage to convert
 * @returns A stringified float
 */
export const toPercentFloatString = (value: string) => (Number(value) / 100).toString();

/**
 * Converts a stringified percentage float (0.0-1.0) to a stringified integer (0-100).
 *
 * @param value - the percentage to convert
 * @returns A stringified integer
 */
export const toPercentIntegerString = (value: string) => Math.round(Number(value) * 100).toString();

/**
 * Converts a decimal number to a pretty representation
 * with fixed decimal places.
 *
 * @param val - a decimal number of string form
 * @param dp - number of decimal places (4 by default ie. 0.0000)
 * @returns A prettified decimal number
 */
export const toDisplay = (val: string | number | Big, dp = 4) => {
  let displayValue;
  try {
    displayValue = Big(val).toFixed(dp);
  } catch (e: any) {
    Console.warn(`${displayValue} not a valid decimal number: ${e}`);
  }
  return displayValue;
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
    Console.warn(`${coin.amount} not a valid decimal number: ${e}`);
  }
  return displayCoin;
};

/**
 * Converts a decimal number of μNYM (micro NYM) to NYM.
 *
 * @param unym - string representation of a decimal number of μNYM
 * @param dp - number of decimal places (4 by default ie. 0.0000)
 * @returns The corresponding decimal number in NYM
 */
export const unymToNym = (unym: string | Big, dp = 4) => {
  let nym;
  try {
    nym = Big(unym).div(1_000_000).toFixed(dp);
  } catch (e: any) {
    Console.warn(`${unym} not a valid decimal number: ${e}`);
  }
  return nym;
};

/**
 *
 * Checks if the user's balance is enough to pay the fee
 * @param balance - The user's current balance
 * @param fee - The fee for the tx
 * @param tx - The amount of the tx
 * @returns boolean
 *
 */

export const isBalanceEnough = (fee: string, tx: string = '0', balance: string = '0') => {
  console.log('balance', balance, fee, tx);
  try {
    return Big(balance).gte(Big(fee).plus(Big(tx)));
  } catch (e) {
    console.log(e);
    return false;
  }
};

export const getIntervalAsDate = async () => {
  const interval = await getCurrentInterval();
  const secondsToNextInterval =
    Number(interval.epochs_in_interval - interval.current_epoch_id) * Number(interval.epoch_length_seconds);

  const nextInterval = format(
    add(new Date(), {
      seconds: secondsToNextInterval,
    }),
    'dd/MM/yyyy, HH:mm',
  );

  const nextEpoch = format(
    add(fromUnixTime(Number(interval.current_epoch_start_unix)), {
      seconds: Number(interval.epoch_length_seconds),
    }),
    'HH:mm',
  );

  return { nextEpoch, nextInterval };
};
