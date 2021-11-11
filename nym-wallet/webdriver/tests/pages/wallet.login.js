class WalletLogin {
  get signInLabel() {
    return $("[data-testid='sign-in']");
  }
  get mnemonic() {
    return $("#mnemonic");
  }
  get signInButton() {
    return $("[type='submit']");
  }
  get errorValidation() {
    return $("[class='MuiAlert-message']");
  }
  get accountBalance() {
    return $("[data-test-id='account-balance']");
  }
  get accountBalanceText() {
    return $("[class='MuiAlert-message']");
  }
  get walletAddress() {
    return $("[data-testid='wallet-address']");
  }

  //login to the application
  enterMnemonic = async (mnemonic) => {
    await this.mnemonic.addValue(mnemonic);
    await this.signInButton.click();
    await this.accountBalance.isExisting();
  };
}
module.exports = new WalletLogin();
