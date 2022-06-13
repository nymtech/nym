class WalletSend {
  get fromAddress() {
    return $("#from");
  }
  get toAddress() {
    return $("#to");
  }
  get amount() {
    return $("#amount");
  }
  get nextButton() {
    return $("[data-testid='button");
  }
  get sendHeader() {
    return $("[data-testid='Send punk']");
  }
  get accountBalance() {
    return $("[data-testid='account-balance']");
  }
  get amountReviewAndSend() {
    return $("[data-testid='Amount']");
  }
  get toAddressReviewAndSend() {
    return $("[data-testid='To']");
  }
  get fromAddressReviewAndSend() {
    return $("[data-testid='From']");
  }
  get transferFeeAmount() {
    return $("[data-testid='Transfer fee']");
  }
  get reviewAndSendBackButton() {
    return $("[data-testid='back-button']");
  }
  get sendButton() {
    return $("[data-testid='button']");
  }
  get transactionComplete() {
    return $("[data-testid='transaction-complete']");
  }
  get transactionCompleteRecipient() {
    return $("[data-testid='to-address']");
  }
  get transactionCompleteAmount() {
    return $("[data-testid='send-amount']");
  }
  get finishButton() {
    return $("[data-testid='button']");
  }
}

module.exports = new WalletSend();
