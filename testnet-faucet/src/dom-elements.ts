// DOM elements

const addressInput = document.getElementById(
  'address-input',
) as HTMLInputElement;
const amountInput = document.getElementById('amount-input') as HTMLInputElement;
const requestTokensBtn = document.getElementById('request-tokens');
const checkBalanceBtn = document.getElementById('check-balance');
const balanceElem = document.querySelector('.balance');
const loadingElem = document.querySelector('.loading');
const errorText = document.querySelector('.error-text');

export {
  addressInput,
  amountInput,
  requestTokensBtn,
  checkBalanceBtn,
  balanceElem,
  loadingElem,
  errorText,
};
