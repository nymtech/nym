import { appWindow } from '@tauri-apps/api/window';
import bs58 from 'bs58';
import { valid } from 'semver';
import { basicRawCoinValueValidation, MajorAmountString } from '@nymproject/types';
import { getLockedCoins, getSpendableCoins, userBalance } from '../requests';
import { Console } from './console';

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
  majorAmountAsString: MajorAmountString,
  minimumAmountAsString: MajorAmountString,
): Promise<boolean> => {
  // tests basic coin value requirements, like no more than 6 decimal places, value lower than total supply, etc
  if (!Number(majorAmountAsString)) {
    return false;
  }

  try {
    if (!basicRawCoinValueValidation(majorAmountAsString)) {
      return false;
    }

    const majorValueFloat = parseInt(majorAmountAsString, Number(10));

    return majorValueFloat >= parseInt(minimumAmountAsString, Number(10));
  } catch (e) {
    Console.error(e as string);
    return false;
  }

  // this conversion seems really iffy but I'm not sure how to better approach it
};

export const isValidHostname = (value: string) => {
  // regex for ipv4 and ipv6 and hhostname- source http://jsfiddle.net/DanielD/8S4nq/
  const hostnameRegex =
    /((^\s*((([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5])\.){3}([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5]))\s*$)|(^\s*((([0-9A-Fa-f]{1,4}:){7}([0-9A-Fa-f]{1,4}|:))|(([0-9A-Fa-f]{1,4}:){6}(:[0-9A-Fa-f]{1,4}|((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3})|:))|(([0-9A-Fa-f]{1,4}:){5}(((:[0-9A-Fa-f]{1,4}){1,2})|:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3})|:))|(([0-9A-Fa-f]{1,4}:){4}(((:[0-9A-Fa-f]{1,4}){1,3})|((:[0-9A-Fa-f]{1,4})?:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9A-Fa-f]{1,4}:){3}(((:[0-9A-Fa-f]{1,4}){1,4})|((:[0-9A-Fa-f]{1,4}){0,2}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9A-Fa-f]{1,4}:){2}(((:[0-9A-Fa-f]{1,4}){1,5})|((:[0-9A-Fa-f]{1,4}){0,3}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9A-Fa-f]{1,4}:){1}(((:[0-9A-Fa-f]{1,4}){1,6})|((:[0-9A-Fa-f]{1,4}){0,4}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(:(((:[0-9A-Fa-f]{1,4}){1,7})|((:[0-9A-Fa-f]{1,4}){0,5}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:)))(%.+)?\s*$))|(^\s*((?=.{1,255}$)(?=.*[A-Za-z].*)[0-9A-Za-z](?:(?:[0-9A-Za-z]|\b-){0,61}[0-9A-Za-z])?(?:\.[0-9A-Za-z](?:(?:[0-9A-Za-z]|\b-){0,61}[0-9A-Za-z])?)*)\s*$)/;

  return hostnameRegex.test(value);
};

export const validateVersion = (version: string): boolean => {
  try {
    return valid(version) !== null;
  } catch (e) {
    return false;
  }
};

export const validateLocation = (location: string): boolean =>
  // right now only perform the stupid check of whether the user copy-pasted the tooltip... (with or without brackets)
  !location.trim().includes('physical location of your node');

export const validateRawPort = (rawPort: number): boolean => !Number.isNaN(rawPort) && rawPort >= 1 && rawPort <= 65535;

export const truncate = (text: string, trim: number) => `${text.substring(0, trim)}...`;

export const isGreaterThan = (a: number, b: number) => a > b;

export const checkHasEnoughFunds = async (allocationValue: string): Promise<boolean> => {
  try {
    const walletValue = await userBalance();

    const remainingBalance = +walletValue.amount.amount - +allocationValue;
    return remainingBalance >= 0;
  } catch (e) {
    Console.log(e as string);
    return false;
  }
};

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
