class WalletLogin {
  get errorValidation(): Promise<WebdriverIO.Element> {
    return $("[data-testid='error']")
  }

  get signInAccount(): Promise<WebdriverIO.Element> {
    return $("[data-testid='signIn']")
  }

  get createAccount(): Promise<WebdriverIO.Element> {
    return $("[data-testid='createAccount']")
  }

  get getMnemonicPhrase(): Promise<WebdriverIO.Element> {
    return $("[data-testid='mnemonic-phrase']")
  }

  get signInButtonReturn(): Promise<WebdriverIO.Element> {
    return $("[data-testid='sign-in-button']")
  }

  get backButton(): Promise<WebdriverIO.Element> {
    return $("[data-testid='backButton']")
  }

  get mnemonic(): Promise<WebdriverIO.Element> {
    return $('#mui-1')
  }

  get selectTextArea(): Promise<WebdriverIO.Element> {
    //bit nasty using the xpath - but it's not liking the elements
    return $("//*[@id='mui-1']")
  }

  get accountBalance(): Promise<WebdriverIO.Element> {
    //bit nasty using the xpath - but it's not liking the elements
    return $("[data-testid='refresh-success']")
  }

  enterMnemonic = async (mnemonic: string): Promise<void> => {
    await (await this.selectTextArea).click()
    await (await this.mnemonic).addValue(mnemonic)
  }
}
export default new WalletLogin()
