import Validator from '@nymproject/nym-validator-client';

(async () => {
  // Environment variables
  const {
    validatorAddress = '',
    mnemonic = '',
    testnetURL1 = '',
    accountAddress = '',
  } = process.env;

  // Connect to the validator client
  const validator = await Validator.connect(
    validatorAddress,
    mnemonic,
    [testnetURL1],
    'punk',
  );

  // DOM elements
  const addressInput = document.getElementById(
    'address-input',
  ) as HTMLInputElement;
  const amountInput = document.getElementById(
    'amount-input',
  ) as HTMLInputElement;
  const requestTokensBtn = document.getElementById('request-tokens');
  const checkBalanceBtn = document.getElementById('check-balance');
  const balanceElem = document.querySelector('.balance');
  const loadingElem = document.querySelector('.loading');
  const errorText = document.querySelector('.error-text');

  // Add event listeners and handlers to button elements
  checkBalanceBtn?.addEventListener('click', async (e) => {
    e.preventDefault();
    setLoading(true);
    showBalance(false);
    showErrorText(false);
    const balance = await getBalance();
    setTimeout(() => {
      setLoading(false);
      displayBalance(balance?.amount);
    }, 1000);
  });

  requestTokensBtn?.addEventListener('click', async (e) => {
    e.preventDefault();
    setLoading(true);
    showBalance(false);
    showErrorText(false);
    if (addressInput.value.length === 0 || accountAddress.length === 0) {
      showErrorText(true);
    }
    try {
      await validator.send(accountAddress, addressInput.value, [
        {
          amount: amountInput.value,
          denom: 'upunk',
        },
      ]);
      displayTransferResult({
        success: true,
        amount: amountInput.value,
        accountAddress,
      });
    } catch (e) {
      displayTransferResult({ success: false, error: e });
    } finally {
      resetInputs();
    }
  });

  // Functions
  const getBalance = async () => {
    return await validator.getBalance(accountAddress);
  };

  const setLoading = (isLoading: boolean) => {
    if (isLoading) {
      loadingElem?.classList.remove('hide');
      checkBalanceBtn?.setAttribute('disabled', 'true');
      requestTokensBtn?.setAttribute('disabled', 'true');
    } else {
      loadingElem?.classList.add('hide');
      checkBalanceBtn?.removeAttribute('disabled');
      requestTokensBtn?.removeAttribute('disabled');
    }
  };

  const showBalance = (show: boolean) =>
    show
      ? balanceElem?.classList.remove('hide')
      : balanceElem?.classList.add('hide');

  const showErrorText = (show: boolean) => {
    show
      ? errorText?.classList.remove('hide')
      : errorText?.classList.add('hide');
  };

  const displayBalance = (balance?: string) => {
    if (!balance && balanceElem) {
      balanceElem.innerHTML = 'Unable to obtain current balance';
    }
    if (balance && balanceElem) {
      balanceElem.innerHTML = `Current balance on the faucet account it <strong>${balance}</strong> upunks`;
    }
    showBalance(true);
  };

  const displayTransferResult = ({
    success,
    amount,
    accountAddress,
    error,
  }: {
    success: boolean;
    amount?: string;
    accountAddress?: string;
    error?: any;
  }) => {
    if (success && balanceElem) {
      balanceElem.innerHTML = `Successfully transfered ${amount!} upunk to address ${accountAddress!}`;
    } else if (!success && balanceElem) {
      balanceElem.innerHTML = `Transfer failed - ${error}`;
    }
    setLoading(false);
    showBalance(true);
  };

  const resetInputs = () => {
    addressInput.value = '';
    amountInput.value = '';
  };
})();
