class WalletReceive {
  get receiveNymHeader(): Promise<WebdriverIO.Element> {
    return $(
      '#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardHeader-root > div > span',
    )
  }
  get receiveNymText(): Promise<WebdriverIO.Element> {
    return $("[data-testid='receive-nym']")
  }
  get walletAddress(): Promise<WebdriverIO.Element> {
    return $("[data-testid='client-address']")
  }
  get copyButton(): Promise<WebdriverIO.Element> {
    return $("[data-testid='copy-button']")
  }
  get qrCode(): Promise<WebdriverIO.Element> {
    return $("[data-testid='qr-code']")
  }

  WaitForButtonChangeOnCopy = async (): Promise<void> => {
    await (await this.copyButton).click()

    await (await this.copyButton).waitForDisplayed({ timeout: 1500 })

    await (
      await this.copyButton
    ).waitUntil(
      async function () {
        return (await this.getText()) === 'COPIED'
      },
      {
        timeout: 1500,
        timeoutMsg: 'expected text to be different after 1.5s',
      },
    )
  }
}

export default new WalletReceive()
