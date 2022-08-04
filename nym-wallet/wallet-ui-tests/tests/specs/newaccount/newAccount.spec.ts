import Auth from '../../pageobjects/authScreens'
import Balance from '../../pageobjects/balanceScreen'
const textConstants = require("../../../common/text-constants");
const userData = require("../../../common/user-data.json");
const deleteScript = require("../../../scripts/deletesavedwallet")
const Helper = require('../../../common/helper');


describe('Create a new account and verify it exists', () => {

    it('generate new mnemonic and verify mnemonic words', async () => {

        // delete any existing saved-wallet.json
        deleteScript
        // click through create account flow
        await Helper.navigateAndClick(Auth.createAccount)
        await Helper.elementVisible(Auth.mnemonicPhrase)
        // save mnemonic phrase
        let mnemonic = await (await Auth.mnemonicPhrase).getText()
        let arrayMnemonic = mnemonic.split(" ")
        await Helper.navigateAndClick(Auth.copyMnemonic)
        await Helper.navigateAndClick(Auth.iSavedMnemonic)
        // verify the mnemonic words in the correct order
        let mnemonicWordTiles = await (await Auth.mnemonicWordTile)
        let wordTileIndex = await (await Auth.wordIndex)

        const wordsArray: any[] = []

        for (const word of mnemonicWordTiles) {
            const wordText = await word.getText()
            const index = arrayMnemonic.indexOf(wordText)
            wordsArray.push({ word, index })
        }

        for (const index of wordTileIndex) {
            const indexValue = await index.getText()
            const match = wordsArray.find((word) => +word.index === +indexValue - 1)
            if (match) {
                await match.word.click()
            }
        }
        // ensure that once the task above is complete, the 'next' button is enabled
        const nextButton = await Auth.nextToStep3
        const isNextDisabled = await nextButton.getAttribute('disabled')
        expect(isNextDisabled).toBe(null)

    })

    it('click skip password', async () => {

        // click on skip password creation
        await Helper.navigateAndClick(Auth.nextToStep3)
        await Helper.navigateAndClick(Auth.skipPasswordAndSignInWithMnemonic)
        // can see mnemonic login page
        await Helper.elementVisible(Auth.mnemonicInput)
        await Helper.navigateAndClick(Auth.backToSignInOptions)

    })

    it('set up invalid password for new account', async () => {

        // enter invalid password in both fields
        await Helper.navigateAndClick(Auth.password)
        await Helper.addValueToTextField(Auth.password, textConstants.incorrectPassword)
        await Helper.navigateAndClick(Auth.confirmPassword)
        await Helper.addValueToTextField(Auth.confirmPassword, textConstants.incorrectPassword)
        // verify that the 'next' button is still disabled
        const nextButton = await Auth.nextStorePassword
        const isNextDisabled = await nextButton.getAttribute('disabled')
        expect(isNextDisabled).toBe("true")

    })

    it('set up valid password for new account', async () => {

        // enter a valid password in both fields
        await Helper.navigateAndClick(Auth.password)
        await Helper.addValueToTextField(Auth.password, textConstants.password)
        await Helper.navigateAndClick(Auth.confirmPassword)
        await Helper.addValueToTextField(Auth.confirmPassword, textConstants.password)
        // verify that the 'next' button is clickable
        const nextButton = await Auth.nextStorePassword
        const isNextDisabled = await nextButton.getAttribute('disabled')
        expect(isNextDisabled).toBe(null)

    })

    it('proceed to login with newly created password', async () => {

        // login with a password
        await Helper.navigateAndClick(Auth.nextStorePassword)
        await Helper.navigateAndClick(Auth.enterPassword)
        await Helper.addValueToTextField(Auth.enterPassword, textConstants.password)
        await Helper.navigateAndClick(Auth.signInPasswordButton)
        // TO-DO for some reason this is failing due to failed to decrypt the wallet etc error
        await Helper.elementVisible(Balance.balance)
        //new accounts will always default to mainnet, so 0 balance
        await Helper.verifyStrictText(Balance.nymBalance, textConstants.noNym)

    })
})