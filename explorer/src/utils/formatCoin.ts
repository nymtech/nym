import { Decimal } from "@cosmjs/math";
import { Coin } from '@nymproject/nym-validator-client';

const thinSpace = "\u202F";

// TO-DO - APT
// Switch out this code for `printableCoin` from validator client
// Method currently throwing dependency error:
// 'Module not found: Can't resolve 'stream' in '/Users/adrianthompson/Documents/nym/explorer/node_modules/cipher-base'
// node_modules/cipher-base/index.js'

export function normaliseCoin(coin: any) {
    if (!coin) {
      return Decimal.fromAtomics('0', 6);
    }
    if (coin.denom.startsWith('u')) {
      return Decimal.fromAtomics(coin.amount, 6);
    } else {
        // @ts-ignore
      return Decimal.fromAtomics(coin.amount, 6).multiply(1000000);
    }
}
  
export function addDecimals(coins: Coin[]) {
    if (!Array.isArray(coins) || !coins.length) {
        return Decimal.fromAtomics('0', 6);
    }
    return coins
      .map(normaliseCoin)
      .reduce((acc, item) => acc.plus(item), Decimal.fromAtomics('0', 6)); // sum
}

export function formatCoin(coin: any) {
    if (!coin) {
        return "0";
    }
    if (coin.denom.startsWith("u")) {
        const ticker = coin.denom.slice(1).toUpperCase();
        return Decimal.fromAtomics(coin.amount, 6).toString() + thinSpace + ticker;
    }
    else {
        return coin.amount + thinSpace + coin.denom;
    }
}
