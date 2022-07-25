import Auth from '../../pageobjects/authScreens'
import Balance from '../../pageobjects/balanceScreen'
const textConstants = require("../../../common/text-constants");
const userData = require("../../../common/user-data.json");


describe('Wallet sign in functionality with mnemonic', () => {

  it('sign in with no mnemonic throws error', async () => {

    //click through sign without entering a mnemonic
    await (await Auth.signInButton).click()
    await (await Auth.signInMnemonic).click()
    await (await Auth.signIn).click()
    // wait for error
    await (await Auth.error).waitForDisplayed({ timeout: 1500 })
    let getErrorWarning = await (await Auth.error).getText()
    // verify error has the correct message
    expect(getErrorWarning).toStrictEqual(textConstants.signInWithoutMnemonic)

  })

  it('sign in with incorrect mnemonic throws error', async () => {

    // await (await Auth.mnemonicInput).waitForDisplayed()
    await (await Auth.mnemonicInput).addValue(textConstants.incorrectMnemonic)
    await (await Auth.signIn).click()
    let getErrorWarning = await (await Auth.error).getText()
    expect(getErrorWarning).toContain(textConstants.signInIncorrectMnemonic)

  })

  it('sign in with random string throws error', async () => {

    // await (await Auth.mnemonicInput).waitForDisplayed()
    await (await Auth.mnemonicInput).addValue(textConstants.randomString)
    await (await Auth.signIn).click()
    let getErrorWarning = await (await Auth.error).getText()
    expect(getErrorWarning).toContain(textConstants.signInRandomString)

  })

  it('should sign in with valid credentials', async () => {

    // go back to create account option
    await (await Auth.backToSignInOptions).click()
    await (await Auth.backToWelcomePage).click()
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

    // TO-DO the balance is not always caputred, some tests fail with ' ' not equal '0 NYM' ??
    let balance = await (await Balance.nymBalance).getText()
    //new accounts will always default to mainnet, so 0 balance
    expect(balance).toStrictEqual(textConstants.noNym)

  })
})