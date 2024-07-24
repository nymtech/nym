class WalletCreate {
  get createAccount() {
    return $("[href='#']");
  }
  get create() {
    return $("[data-testid='create-button']");
  }
  get accountCreatedSuccessfully() {
    return $("[data-testid='mnemonic-warning']");
  }
  get walletMnemonicValue() {
    return $("[data-testid='mnemonic-phrase']");
  }
  get punkAddress() {
    return $("[data-testid='wallet-address']");
  }
  get backToSignIn() {
    return $("[data-testid='sign-in-button']");
  }
  get signInButton() {
    return $("[type='submit']");
  }
}
module.exports = new WalletCreate();
