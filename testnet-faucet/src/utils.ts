import { printableBalanceToNative } from '@nymproject/nym-validator-client/dist/currency'

export const getCoinValue = (raw: string): number => {
  let native = printableBalanceToNative(raw)
  return parseInt(native)
}

export const validAmount = (amount: string): boolean => {
  if (isNaN(+amount)) return false
  return true
}
