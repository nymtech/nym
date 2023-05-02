import Big from 'big.js';

export const unymToNym = (unym: string | Big, dp = 4) => {
  let nym;
  try {
    nym = Big(unym).div(1_000_000).toFixed(dp);
  } catch (e: any) {
    console.warn(`${unym} not a valid decimal number: ${e}`);
  }
  return nym;
};

export const nymToUnym = (nym: string) => {
  let unym;
  try {
    unym = Big(nym).mul(1_000_000).toString();
  } catch (e: any) {
    console.warn(`unable to convert nym to unym: ${e}`);
  }
  return unym;
};
