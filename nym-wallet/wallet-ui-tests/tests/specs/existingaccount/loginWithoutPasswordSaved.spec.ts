import Auth from '../../pageobjects/authScreens'
import Balance from '../../pageobjects/balanceScreen'
const textConstants = require("../../../common/text-constants");
const userData = require("../../../common/user-data.json");
const deleteWallet = require("../../../scripts/deletesavedwallet");
const walletExists = require("../../../scripts/savedwalletexists")
const { exec } = require("child_process")


describe('Wallet sign in functionality without creating password', () => {

    it('sign in with invalid password and no saved wallet.json file throws error', async () => {
        //click through sign without entering a password
        await (await Auth.signInButton).click()
        await (await Auth.signInPassword).click()

        // enter invalid password
        await (await Auth.enterPassword).addValue(textConstants.incorrectPassword)
        await (await Auth.signInPasswordButton).click()
        // wait for error
        await (await Auth.error).waitForDisplayed({ timeout: 1500 })
        let getErrorWarning = await (await Auth.error).getText()
        expect(getErrorWarning).toStrictEqual(textConstants.failedToFindWalletFile)

    })

})