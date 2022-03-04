class WalletSend {
  get fromAddress() {
    return $('#from')
  }
  get toAddress(): Promise<WebdriverIO.Element> {
    return $('#to')
  }
  get amount(): Promise<WebdriverIO.Element> {
    return $('#amount')
  }
  get nextButton(): Promise<WebdriverIO.Element> {
    return $("[data-testid='button")
  }
  get sendHeader(): Promise<WebdriverIO.Element> {
    return $("[data-testid='Send punk']")
  }
  get accountBalance(): Promise<WebdriverIO.Element> {
    return $("[data-testid='account-balance']")
  }
  get amountReviewAndSend(): Promise<WebdriverIO.Element> {
    return $("[data-testid='Amount']")
  }
  get toAddressReviewAndSend(): Promise<WebdriverIO.Element> {
    return $("[data-testid='To']")
  }
  get fromAddressReviewAndSend(): Promise<WebdriverIO.Element> {
    return $("[data-testid='From']")
  }
  get transferFeeAmount(): Promise<WebdriverIO.Element> {
    return $("[data-testid='Transfer fee']")
  }
  get reviewAndSendBackButton(): Promise<WebdriverIO.Element> {
    return $("[data-testid='back-button']")
  }
  get sendButton(): Promise<WebdriverIO.Element> {
    return $("[data-testid='button']")
  }
  get transactionComplete(): Promise<WebdriverIO.Element> {
    return $("[data-testid='transaction-complete']")
  }
  get transactionCompleteRecipient(): Promise<WebdriverIO.Element> {
    return $("[data-testid='to-address']")
  }
  get transactionCompleteAmount(): Promise<WebdriverIO.Element> {
    return $("[data-testid='send-amount']")
  }
  get finishButton(): Promise<WebdriverIO.Element> {
    return $("[data-testid='button']")
  }
}

export default new WalletSend()
