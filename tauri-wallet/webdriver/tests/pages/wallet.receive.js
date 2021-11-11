class WalletReceive {
  get receiveNymHeader() {
    return $(
      "#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardHeader-root > div > span"
    );
  }
  get receiveNymText() {
    return $("[data-testid='receive-nym']");
  }
  get walletAddress() {
    return $("[data-testid='client-address']");
  }
  get copyButton() {
    return $("[data-testid='copy-button']");
  }
  get qrCode() {
    return $("[data-testid='qr-code']");
  }

  WaitForButtonChangeOnCopy = async () => {
    await this.copyButton.click();

    await this.copyButton.waitForDisplayed({ timeout: 1500 });

    await this.copyButton.waitUntil(
      async function () {
        return (await this.getText()) === "COPIED";
      },
      {
        timeout: 1500,
        timeoutMsg: "expected text to be different after 1.5s",
      }
    );
  };
}

module.exports = new WalletReceive();
