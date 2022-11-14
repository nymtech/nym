import Auth from '../../pageobjects/authScreens'
const deleteScript = require("../../../scripts/deletesavedwallet")
const walletExists = require("../../../scripts/savedwalletexists")
const Helper = require('../../../common/helper');


export function newMnemonicLogin() {
    
  // TO-DO find the best approach

describe('Create a new account and verify login', () => {

    it('generate new mnemonic and verify mnemonic words', async () => {

        // delete an existing saved-wallet.json
        walletExists
        deleteScript
        // click through create account flow
        console.log("--------------------")
        console.log("step1")
        await Helper.navigateAndClick(Auth.createAccount)
        await browser.pause(1500)
        console.log("--------------------")
        console.log("step2")
        // save and verify mnemonic
        const mnemonic = await browser.execute(() => {
            return document.getElementById("mnemonicPhrase").innerHTML;
        });
        await Helper.mnemonicWordTileIndex(mnemonic)
        // ensure that once the task above is complete, the 'next' button is enabled
        const nextButton = await Auth.nextToStep3
        const isNextDisabled = await nextButton.getAttribute('disabled')
        expect(isNextDisabled).toBe(null)
        await Helper.navigateAndClick(Auth.nextToStep3)

    })
})
}