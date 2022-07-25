import Auth from '../../pageobjects/authScreens'
import Balance from '../../pageobjects/balanceScreen'
const textConstants = require("../../../common/text-constants");
const userData = require("../../../common/user-data.json");
const deleteWallet = require("../../../scripts/deletesavedwallet");
const walletExists = require("../../../scripts/savedwalletexists")

describe('Wallet sign in functionality with password', () => {

    it('sign in with no password throws error', async () => {

        //click through sign without entering a password
        await (await Auth.signInButton).click()
        await (await Auth.signInPassword).click()
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

        // TO-DO GET BACK TO THIS LOGIC. It's currently only searching for the second condition (if the wallet json file doesn't exist)
        const scriptExist = walletExists.doesFileExist
        scriptExist ? expect(getErrorWarning).toStrictEqual(textConstants.invalidPasswordOnSignIn) : expect(getErrorWarning).toStrictEqual(textConstants.failedToFindWalletFile)

    })

    it('sign in with invalid password and no saved wallet.json file throws error', async () => {

        // shell script to remove the json file
        deleteWallet.deleteSavedFile
        // enter invalid password
        await (await Auth.enterPassword).addValue(textConstants.incorrectPassword)
        await (await Auth.signInPasswordButton).click()
        // wait for error
        await (await Auth.error).waitForDisplayed({ timeout: 1500 })
        let getErrorWarning = await (await Auth.error).getText()
        expect(getErrorWarning).toStrictEqual(textConstants.failedToFindWalletFile)

    })

})