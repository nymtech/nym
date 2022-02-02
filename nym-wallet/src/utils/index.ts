import { invoke } from '@tauri-apps/api'
import bs58 from 'bs58'
import { minor, valid } from 'semver'
import { userBalance, majorToMinor, getGasFee } from '../requests'
import { Coin, Network, TCurrency } from '../types'

export const validateKey = (key: string, bytesLength: number): boolean => {
  // it must be a valid base58 key
  try {
    const bytes = bs58.decode(key)
    // of length 32
    return bytes.length === bytesLength
  } catch (e) {
    console.error(e)
    return false
  }
}

export const validateAmount = async (amount: string, minimum: string): Promise<boolean> => {
  // tests basic coin value requirements, like no more than 6 decimal places, value lower than total supply, etc
  if (!Number(amount)) {
    return false
  }

  try {
    const minorValueStr: Coin = await invoke('major_to_minor', {
      amount,
    })

    if (!basicRawCoinValueValidation(minorValueStr.amount)) {
      return false
    }

    const minorValue = parseInt(minorValueStr.amount)

    return minorValue >= parseInt(minimum)
  } catch (e) {
    console.log(e)
    return false
  }

  // this conversion seems really iffy but I'm not sure how to better approach it
}

export const basicRawCoinValueValidation = (rawAmount: string): boolean => {
  const amountFloat = parseFloat(rawAmount)

  // it cannot have more than 6 decimal places
  if (amountFloat !== parseInt(amountFloat.toFixed(6))) {
    return false
  }

  // it cannot be larger than the total supply
  if (amountFloat > 1_000_000_000_000_000) {
    return false
  }

  // it can't be lower than one micro coin
  return amountFloat >= 0.000001
}

export const isValidHostname = (value: string) => {
  // regex for ipv4 and ipv6 and hhostname- source http://jsfiddle.net/DanielD/8S4nq/
  const hostnameRegex =
    /((^\s*((([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5])\.){3}([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5]))\s*$)|(^\s*((([0-9A-Fa-f]{1,4}:){7}([0-9A-Fa-f]{1,4}|:))|(([0-9A-Fa-f]{1,4}:){6}(:[0-9A-Fa-f]{1,4}|((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3})|:))|(([0-9A-Fa-f]{1,4}:){5}(((:[0-9A-Fa-f]{1,4}){1,2})|:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3})|:))|(([0-9A-Fa-f]{1,4}:){4}(((:[0-9A-Fa-f]{1,4}){1,3})|((:[0-9A-Fa-f]{1,4})?:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9A-Fa-f]{1,4}:){3}(((:[0-9A-Fa-f]{1,4}){1,4})|((:[0-9A-Fa-f]{1,4}){0,2}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9A-Fa-f]{1,4}:){2}(((:[0-9A-Fa-f]{1,4}){1,5})|((:[0-9A-Fa-f]{1,4}){0,3}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9A-Fa-f]{1,4}:){1}(((:[0-9A-Fa-f]{1,4}){1,6})|((:[0-9A-Fa-f]{1,4}){0,4}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(:(((:[0-9A-Fa-f]{1,4}){1,7})|((:[0-9A-Fa-f]{1,4}){0,5}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:)))(%.+)?\s*$))|(^\s*((?=.{1,255}$)(?=.*[A-Za-z].*)[0-9A-Za-z](?:(?:[0-9A-Za-z]|\b-){0,61}[0-9A-Za-z])?(?:\.[0-9A-Za-z](?:(?:[0-9A-Za-z]|\b-){0,61}[0-9A-Za-z])?)*)\s*$)/

  return hostnameRegex.test(value)
}

export const validateVersion = (version: string): boolean => {
  try {
    const minorVersion = minor(version)
    const validVersion = valid(version)
    return validVersion !== null && minorVersion >= 11
  } catch (e) {
    return false
  }
}

export const validateLocation = (location: string): boolean => {
  // right now only perform the stupid check of whether the user copy-pasted the tooltip... (with or without brackets)
  return !location.trim().includes('physical location of your node')
}

export const validateRawPort = (rawPort: number): boolean => !isNaN(rawPort) && rawPort >= 1 && rawPort <= 65535

export const truncate = (text: string, trim: number) => text.substring(0, trim) + '...'

export const isGreaterThan = (a: number, b: number) => a > b

export const checkHasEnoughFunds = async (allocationValue: string) => {
  try {
    const walletValue = await userBalance()
    const minorValue = await majorToMinor(allocationValue)
    const remainingBalance = +walletValue.coin.amount - +minorValue.amount
    return isGreaterThan(remainingBalance, 0)
  } catch (e) {
    console.log(e)
  }
}

export const randomNumberBetween = (min: number, max: number) => {
  min = Math.ceil(min)
  max = Math.floor(max)
  return Math.floor(Math.random() * (max - min + 1) + min)
}

export const currencyMap = (network: Network) => {
  let currency = {
    minor: 'unym',
    major: 'nym',
  } as TCurrency

  switch (network) {
    case 'MAINNET':
      currency.minor = 'unym'
      currency.major = 'nym'
      break
    case 'SANDBOX':
      currency.minor = 'unymt'
      currency.major = 'nymt'
      break
  }

  return currency
}
