import Auth from '../../pageobjects/authScreens'
import Balance from '../../pageobjects/balanceScreen'
import ValidatorClient from '@nymproject/nym-validator-client';
const textConstants = require("../../../common/text-constants");
const userData = require("../../../common/user-data.json");
const { exec } = require("child_process")


describe('Create password for existing account and use it to sign in', () => {

    it('enter incorrect mnemonic', async () => {

        //click through sign in
        await (await Auth.signInButton).click()
        await (await Auth.signInMnemonic).click()
        //instead of entering mnemonic, click on create a password
        await (await Auth.createPassword).click()
        //enter incorrect  mnemonic
        await (await Auth.mnemonicInput).addValue(textConstants.incorrectMnemonic)
        await (await Auth.nextToPasswordCreation).click()
        // assert error message is correct
        let getErrorWarning = await (await Auth.error).getText()
        expect(getErrorWarning).toStrictEqual(textConstants.incorrectMnemonicPasswordCreation)

    })

    it('enter random string', async () => {

        // enter random string as mnemonic
        await (await Auth.mnemonicInput).addValue(textConstants.randomString)
        await (await Auth.nextToPasswordCreation).click()
        // assert error is correct
        let getErrorWarning = await (await Auth.error).getText()
        expect(getErrorWarning).toStrictEqual(textConstants.incorrectMnemonicPasswordCreation)

    })


    it('enter correct mnemonic', async () => {

        // generate random mnemonic in the backend
        const randomMnemonic = ValidatorClient.randomMnemonic();
        // use it to continue with password creation flow
        await (await Auth.backToMnemonicSignIn).click()
        await (await Auth.createPassword).click()
        await (await Auth.mnemonicInput).addValue(randomMnemonic)
        await (await Auth.nextToPasswordCreation).click()
        await (await Auth.password).waitForDisplayed({ timeout: 2500 })

    })

    it('create an invalid password', async () => {
        // type an invalid password in both fields
        await (await Auth.password).addValue(textConstants.incorrectPassword)
        await (await Auth.confirmPassword).click({timeout: 1500})
        await (await Auth.confirmPassword).addValue(textConstants.incorrectPassword)
        // ensure the button to proceed is still disabled 
        const nextButton = await Auth.createPasswordButton
        const isNextDisabled = await nextButton.getAttribute('disabled')
        expect(isNextDisabled).toBe("true")

    })

    it('create a valid password', async () =>{
        await (await Auth.password).click({timeout: 1500})
        await (await Auth.password).addValue(textConstants.password)
        await (await Auth.confirmPassword).click({timeout: 1500})
        await (await Auth.confirmPassword).addValue(textConstants.password)
        await (await Auth.createPasswordButton).click()

    })

    it('sign in with no password throws error', async () => {

        //click sign without entering a password
        await (await Auth.signInPasswordButton).click()
        // wait for error
        await (await Auth.error).waitForDisplayed({ timeout: 1500 })
        let getErrorWarning = await (await Auth.error).getText()
        // verify error has the correct message
        expect(getErrorWarning).toStrictEqual(textConstants.signInWithoutPassword)

    })

    it('sign in with invalid password throws error', async () => {

        // enter invalid password
        await (await Auth.enterPassword).addValue(textConstants.incorrectPassword)
        await (await Auth.signInPasswordButton).click()
        // wait for error
        await (await Auth.error).waitForDisplayed({ timeout: 1500 })
        let getErrorWarning = await (await Auth.error).getText()
        expect(getErrorWarning).toStrictEqual(textConstants.invalidPasswordOnSignIn)
        
    })

})