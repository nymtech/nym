import Auth from '../../pageobjects/authScreens'
import Balance from '../../pageobjects/balanceScreen'
import ValidatorClient from '@nymproject/nym-validator-client';
import { text } from 'stream/consumers';
const textConstants = require("../../../common/text-constants");
const userData = require("../../../common/user-data.json");
const Helper = require('../../../common/helper');



describe('Wallet sign in functionality with mnemonic', () => {

  it('get to the sign in with mnemonic screen', async () => {

    // click through to reach the mnemonic sign in
    await Helper.navigateAndClick(Auth.signInButton)
    await Helper.navigateAndClick(Auth.signInMnemonic)
    // verify you are on the right screen by confirming the header
    await Helper.verifyStrictText(Auth.mnemonicLoginScreenHeader, textConstants.mnemonicSignIn)

  })

  it('sign in with no mnemonic throws error', async () => {

    await Helper.navigateAndClick(Auth.signIn)
    // wait for error
    await Helper.elementVisible(Auth.error)
    // verify error has the correct message
    await Helper.verifyStrictText(Auth.error, textConstants.signInWithoutMnemonic)

  })

  it('sign in with incorrect mnemonic throws error', async () => {

    // enter an incorrect mnemonic string
    await Helper.addValueToTextField(Auth.mnemonicInput, textConstants.incorrectMnemonic)
    await Helper.navigateAndClick(Auth.signIn)
    // verifty error message is correct
    await Helper.verifyPartialText(Auth.error, textConstants.signInIncorrectMnemonic)

  })

  it('sign in with random string throws error', async () => {

    // enter a random string not in mnemonic "format"
    await Helper.addValueToTextField(Auth.mnemonicInput, textConstants.randomString)
    await Helper.navigateAndClick(Auth.signIn)
    // verifty error message is correct
    await Helper.verifyPartialText(Auth.error, textConstants.signInRandomString)

  })

  it('should sign in with valid credentials', async () => {

    // create new mnemonic
    const randomMnemonic = ValidatorClient.randomMnemonic();
    // enter mnemonic
    await Helper.addValueToTextField(Auth.mnemonicInput, randomMnemonic)
    await Helper.navigateAndClick(Auth.signIn)
    // verify successful login, balance is visible
    await Helper.elementVisible(Balance.balance)
    //new accounts will always default to mainnet, so 0 balance
    // TO-DO this value sometimes returns " " instead of "0"
    await Helper.verifyStrictText(Balance.nymBalance, textConstants.noNym)

  })
})