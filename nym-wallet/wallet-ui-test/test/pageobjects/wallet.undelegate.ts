class WallentUndelegate {
  get transactionFee(): Promise<WebdriverIO.Element> {
    return $("[data-testid='fee-amount']")
  }
  get mixNodeRadioButton(): Promise<WebdriverIO.Element> {
    return $("[value='mixnode']")
  }
  get gatewayRadionButton(): Promise<WebdriverIO.Element> {
    return $("[value='gateway']")
  }
  get nodeIdentity(): Promise<WebdriverIO.Element> {
    return $('#mui-55011')
  }
  get identityHelper(): Promise<WebdriverIO.Element> {
    return $('#identity-helper-text')
  }
  get delegateButton(): Promise<WebdriverIO.Element> {
    return $("[data-testid='submit-button']")
  }
}

export default new WallentUndelegate()
