import walletLogin from '../pageobjects/wallet.login'

describe('Wallet login functionality', () => {
  it.skip('submitting with no mnemonic throws error', async () => {
    //sign into account
    await (await walletLogin.signInAccount).click()

    //submit sign in with no mnemominc
    await (await walletLogin.signInAccount).click()

    await (await walletLogin.errorValidation).waitForDisplayed({ timeout: 1500 })
    let getErrorWarning = await (await walletLogin.errorValidation).getText()

    await (await walletLogin.backButton).click()

    //assert that the error was thrown
    const errorText = 'mnemonic has a word count that is not a multiple of 6: 0'
    expect(getErrorWarning).toStrictEqual(errorText)
  })

  it('should login with valid credentials', async () => {
    //create account
    await (await walletLogin.createAccount).click()

    //allow time for the api to create the wallet address
    await browser.pause(500)
    await (await walletLogin.getMnemonicPhrase).waitForDisplayed({ timeout: 1500 })

    //retrieve mnemonic - copy
    let mnemonicPhrase = await (await walletLogin.getMnemonicPhrase).getText()

    await (await walletLogin.signInButtonReturn).click()

    //input new wallet mnemonic and be inside the app
    await walletLogin.enterMnemonic(mnemonicPhrase)

    await (await walletLogin.signInAccount).click()

    await (await walletLogin.accountBalance).waitForDisplayed({ timeout: 1500 })
    let balance = await (await walletLogin.accountBalance).getText()
    //new accounts will always default to mainnet
    expect(balance).toStrictEqual('0 NYM')
  })
})
