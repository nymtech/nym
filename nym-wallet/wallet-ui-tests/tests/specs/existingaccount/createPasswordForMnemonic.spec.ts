import Auth from '../../pageobjects/authScreens'
import Balance from '../../pageobjects/balanceScreen'
import ValidatorClient from '@nymproject/nym-validator-client';
const textConstants = require("../../../common/text-constants");
const userData = require("../../../common/user-data.json");


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

        await (await Auth.mnemonicInput).addValue(textConstants.randomString)
        await (await Auth.nextToPasswordCreation).click()
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
        await (await Auth.confirmPassword).click()
        await (await Auth.confirmPassword).addValue(textConstants.incorrectPassword)
        // ensure the button to proceed is still disabled 
        // TO-DO the next button seems to not be clickable here, despite the locator id existing in the right place
        const nextButton = await Auth.next
        const isNextDisabled = await nextButton.getAttribute('disabled')
        expect(isNextDisabled).toBe(true)

    })

})