class WalletLogin {
  get signInLabel(): Promise<WebdriverIO.Element> {
    return $("[data-testid='sign-in']")
  }
  get mnemonic(): Promise<WebdriverIO.Element> {
    return $('#mnemonic')
  }
  get signInButton(): Promise<WebdriverIO.Element> {
    return $("[type='submit']")
  }

  get errorValidation(): Promise<WebdriverIO.Element> {
    return $("[class='MuiAlert-message']")
  }

  enterMnemonic = async (mnemonic: string): Promise<void> => {
    await (await this.mnemonic).addValue(mnemonic)
    await (await this.signInButton).click()
  }
}
export default new WalletLogin()
