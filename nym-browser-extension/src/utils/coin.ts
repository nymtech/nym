import Big from 'big.js';

export const unymToNym = (unym: number | Big, dp = 4) => {
  let nym;
  try {
    nym = Big(unym).div(1_000_000).toFixed(dp);
  } catch (e: any) {
    console.warn(`${unym} not a valid decimal number: ${e}`);
  }
  return Number(nym);
};

export const nymToUnym = (nym: number) => {
  let unym;
  try {
    unym = Big(nym).mul(1_000_000);
  } catch (e: any) {
    console.warn(`unable to convert nym to unym: ${e}`);
  }
  return Number(unym);
};
