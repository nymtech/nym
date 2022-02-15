import { printableBalanceToNative } from '@nymproject/nym-validator-client/dist/currency'

export const getCoinValue = (raw: string): number => {
  const native = printableBalanceToNative(raw)
  return parseInt(native)
}

export const validAmount = (amount: string): boolean => {
  if (isNaN(+amount)) return false
  if (+amount > 101) return false
  return true
}

export const handleError = (error: Error) => {
  if (error.message.includes('invalid address'))
    return 'Invalid address. Please check and try again'
  if (error.message.includes('insufficient funds'))
    return 'The faucet is running dry. Please try again in a minute'
}
