class WalletHome {
  get balanceCheck() {
    return $(
      "#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardHeader-root > div > span"
    );
  }
  get punkBalance() {
    return $("");
  }
  get punkAddress() {
    return $("[data-testid='wallet-address']");
  }
  get accountBalance() {
    return $("[data-testid='account-balance']");
  }
  get balanceButton() {
    return $("[href='/balance']");
  }
  get sendButton() {
    return $("[href='/send']");
  }
  get receiveButton() {
    return $("[href='/receive']");
  }
  get bondButton() {
    return $("[href='/bond']");
  }
  get unBondButton() {
    return $("[href='/unbond']");
  }
  get delegateButton() {
    return $("[href='/delegate']");
  }
  get unDelegateButton() {
    return $("[href='/undelegate']");
  }
  get logOutButton() {
    return $("[data-testid='log-out']");
  }
}

module.exports = new WalletHome();
