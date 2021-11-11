class WalletDelegate {
  get header() {
    return $("[data-testid='Delegate']");
  }
  get nodeIdentity() {
    return $("#identity");
  }
  get amountToDelegate() {
    return $("#amount");
  }
  get identityValidation() {
    return $("#identity-helper-text");
  }
  get amountToDelegateValidation() {
    return $("#amount-helper-text");
  }
  get delegateStakeButton() {
    return $("[data-testid='delegate-button']");
  }
  get mixNodeRadioButton() {
    return $("[data-testid='mix-node']");
  }
  get gateWayRadioButton() {
    return $("[data-testid='gate-way']");
  }
  get successfullyDelegate() {
    return $("[data-testid='delegate-success']");
  }
  get finishButton() {
    return $("[data-testid='finish-button']");
  }
  get transactionFeeAmount() {
    return $("[data-testid='fee-amount']");
  }
  get accountBalance() {
    return $("[data-testid='account-balance']");
  }

  //Undelegate
  get unDelegateHeader() {
    return $("[data-testid='Undelegate']");
  }
  get unNodeIdentity() {
    return $("[name='identity']");
  }
  get unDelegateFeeText() {
    return $("[data-testid='fee-amount']");
  }
  get unDelegateGatewayRadioButton() {
    return $("[data-testid='gate-way']");
  }
  get unMixNodeRadioButton() {
    return $("[data-testid='mix-node']");
  }
  get unDelegateButton() {
    return $("[data-testid='submit-button']");
  }
}

module.exports = new WalletDelegate();
