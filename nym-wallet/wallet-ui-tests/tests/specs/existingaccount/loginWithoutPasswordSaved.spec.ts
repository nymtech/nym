import Auth from '../../pageobjects/authScreens'
import Balance from '../../pageobjects/balanceScreen'
const textConstants = require("../../../common/text-constants");
const userData = require("../../../common/user-data.json");
const deleteWallet = require("../../../scripts/deletesavedwallet");
const walletExists = require("../../../scripts/savedwalletexists")
const Helper = require('../../../common/helper');


describe('Wallet sign in functionality without creating password', () => {

    it('sign in with invalid password and no saved wallet.json file throws error', async () => {

        // delete existing saved wallet file
        deleteWallet
        //click through sign without entering a password
        await Helper.navigateAndClick(Auth.signInButton)
        await Helper.navigateAndClick(Auth.signInPassword)
        // enter invalid password
        await Helper.addValueToTextField(Auth.enterPassword,textConstants.incorrectPassword)
        await Helper.navigateAndClick(Auth.signInPasswordButton)
        // wait for error
        await Helper.elementVisible(Auth.error)
        // verify error has the correct message
        await Helper.verifyStrictText(Auth.error, textConstants.failedToFindWalletFile)

    })

})