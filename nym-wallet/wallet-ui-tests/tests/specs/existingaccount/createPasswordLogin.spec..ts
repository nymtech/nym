import Auth from '../../pageobjects/authScreens'
import Balance from '../../pageobjects/balanceScreen'
import ValidatorClient from '@nymproject/nym-validator-client';
const deleteScript = require("../../../scripts/deletesavedwallet")
const textConstants = require("../../../common/text-constants");
const userData = require("../../../common/user-data.json");
    const Helper = require('../../../common/helper');


describe('Create password for existing account and use it to sign in', () => {

    it('enter incorrect mnemonic', async () => {

        //click through sign in
        await Helper.navigateAndClick(Auth.signInButton)
        await Helper.navigateAndClick(Auth.signInMnemonic)
        //instead of entering mnemonic, click on create a password
        await Helper.navigateAndClick(Auth.createPassword)
        //enter incorrect  mnemonic
        await Helper.addValueToTextField(Auth.mnemonicInput, textConstants.incorrectMnemonic)
        await Helper.navigateAndClick(Auth.nextToPasswordCreation)

        // assert error message is correct
        await Helper.verifyStrictText(Auth.error, textConstants.incorrectMnemonicPasswordCreation)
    })

    it('enter random string', async () => {

        // enter random string as mnemonic
        await Helper.addValueToTextField(Auth.mnemonicInput, textConstants.randomString)
        await Helper.navigateAndClick(Auth.nextToPasswordCreation)
        // assert error is correct
        await Helper.verifyStrictText(Auth.error, textConstants.incorrectMnemonicPasswordCreation)

    })


    it('enter correct mnemonic', async () => {

        // generate random mnemonic in the backend
        const randomMnemonic = ValidatorClient.randomMnemonic();
        deleteScript
        // use it to continue with password creation flow
        await Helper.navigateAndClick(Auth.backToMnemonicSignIn)
        await Helper.navigateAndClick(Auth.createPassword)
        await Helper.addValueToTextField(Auth.mnemonicInput, randomMnemonic)
        await Helper.navigateAndClick(Auth.nextToPasswordCreation)
        await Helper.elementVisible(Auth.password)
    })

    it('create an invalid password', async () => {

        // type an invalid password in both fields
        await Helper.addValueToTextField(Auth.password, textConstants.incorrectPassword)
        await Helper.navigateAndClick(Auth.confirmPassword)
        await Helper.addValueToTextField(Auth.confirmPassword, textConstants.incorrectPassword)
        // ensure the button to proceed is still disabled 
        const nextButton = await Auth.createPasswordButton
        const isNextDisabled = await nextButton.getAttribute('disabled')
        expect(isNextDisabled).toBe("true")

    })

    it('create a valid password', async () => {

        // type a valid password in both fields
        await Helper.navigateAndClick(Auth.password)
        await Helper.addValueToTextField(Auth.password, textConstants.password)
        await Helper.navigateAndClick(Auth.confirmPassword)
        await Helper.addValueToTextField(Auth.confirmPassword, textConstants.password)
        // verify the password is created and the next screen is visible
        await Helper.navigateAndClick(Auth.createPasswordButton)
        await Helper.verifyStrictText(Auth.passwordLoginScreenHeader, textConstants.passwordSignIn)

    })

    it('sign in with no password throws error', async () => {

        //click sign without entering a password
        await Helper.navigateAndClick(Auth.signInPasswordButton)
        // wait for error
        await Helper.elementVisible(Auth.error)
        // verify error has the correct message
        await Helper.verifyStrictText(Auth.error, textConstants.signInWithoutPassword)

    })

    it('sign in with invalid password throws error', async () => {

        // enter invalid password
        await Helper.addValueToTextField(Auth.enterPassword, textConstants.incorrectPassword)
        await Helper.navigateAndClick(Auth.signInPasswordButton)
        // wait for error
        await Helper.elementVisible(Auth.error)
        await Helper.verifyStrictText(Auth.error, textConstants.invalidPasswordOnSignIn)

    })

})