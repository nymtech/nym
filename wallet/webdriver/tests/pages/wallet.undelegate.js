class WallentUndelegate {
  get transactionFee() {
    return $("[data-testid='fee-amount']");
  }
  get mixNodeRadioButton() {
    return $("[value='mixnode']");
  }
  get gatewayRadionButton() {
    return $("[value='gateway']");
  }
  get nodeIdentity() {
    return $("#mui-55011");
  }
  get identityHelper() {
    return $("#identity-helper-text");
  }
  get delegateButton() {
    return $("[data-testid='submit-button']");
  }
}

module.exports = new WallentUndelegate();
