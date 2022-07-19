import Auth from '../../pageobjects/authScreens'
import Balance from '../../pageobjects/balanceScreen'

describe('Wallet login functionality', () => {
  it('sign in with no mnemonic throws error', async () => {
    //click through sign without entering a mnemonic
    await (await Auth.signInButton).click()
    await (await Auth.signInMnemonic).click()
    await (await Auth.signIn).click()
    // wait for error
    await (await Auth.error).waitForDisplayed({ timeout: 1500 })
    let getErrorWarning = await (await Auth.error).getText()
    const errorText = 'A mnemonic must be provided'
    // verify error has the correct message
    expect(getErrorWarning).toStrictEqual(errorText)

    // TO-DO fix the below so there's no need to go back before the following test
    await (await Auth.backToSignInOptions).click()
    await (await Auth.backToWelcomePage).click()

  })

  it('should login with valid credentials', async () => {
    // create new mnemonic
    await (await Auth.createAccount).click()
    let mnemonic = await (await Auth.mnemonicPhrase).getText()
    await (await Auth.backToWelcomePageFromCreate).click()
    // back on login page then enter the mnemonic
    await (await Auth.signInButton).click()
    await (await Auth.signInMnemonic).click()
    await (await Auth.mnemonicInput).waitForDisplayed()
    await (await Auth.mnemonicInput).addValue(mnemonic)
    await (await Auth.signIn).click()
    // verify successful login, balance is visible
    await (await Balance.balance).waitForDisplayed({ timeout: 4000 })

    // TO-DO the balance is not always caputred, some tests fail with ' ' not equal '0 NYM'
    let balance = await (await Balance.nymBalance).getText()
    console.log('amountttttttt' + balance)
    //new accounts will always default to mainnet, so 0 balance
    expect(balance).toStrictEqual('0 NYM')
  })
})