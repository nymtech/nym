import Auth from '../../pageobjects/authScreens'
import Balance from '../../pageobjects/balanceScreen'
const textConstants = require("../../../common/text-constants");
const userData = require("../../../common/user-data.json");


describe('Create a new account and verify it exists', () => {

    it('generate new mnemonic and verify mnemonic words', async () => {

        await (await Auth.createAccount).click()
        await (await Auth.mnemonicPhrase).waitForDisplayed({ timeout: 2500 })
        let mnemonic = await (await Auth.mnemonicPhrase).getText()
        let arrayMnemonic = mnemonic.split(" ")
        await (await Auth.copyMnemonic).click()
        await (await expect(Auth.iSavedMnemonic).toBeClickable())
        await (await Auth.iSavedMnemonic).click()

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

        const nextButton = await Auth.nextToStep3
        const isNextDisabled = await nextButton.getAttribute('disabled')
        expect(isNextDisabled).toBe(null)

    })

    it('click skip password', async () => {
        await (await Auth.nextToStep3).click()
        await (await Auth.skipPasswordAndSignInWithMnemonic).click()
        await (await Auth.mnemonicInput).waitForDisplayed({ timeout: 1500 })
        await (await Auth.backToSignInOptions).click()
    })

    it('set up invalid password for new account', async () => {
        await (await Auth.password).click()
        await (await Auth.password).addValue(textConstants.incorrectPassword)
        await (await Auth.confirmPassword).click()
        await (await Auth.confirmPassword).addValue(textConstants.incorrectPassword)
        const nextButton = await Auth.next
        const isNextDisabled = await nextButton.getAttribute('disabled')
        expect(isNextDisabled).toBe("true")

    })

    it('set up valid password for new account', async () => {
        await (await Auth.password).click()
        await (await Auth.password).addValue(textConstants.password)
        await (await Auth.confirmPassword).click()
        await (await Auth.confirmPassword).addValue(textConstants.password)
        const nextButton = await Auth.next
        const isNextDisabled = await nextButton.getAttribute('disabled')
        expect(isNextDisabled).toBe(null)
    })

    it('proceed to login with newly created password', async () => {
        await (await Auth.next).click()
        await (await Auth.enterPassword).waitForDisplayed({ timeout: 1500 })
        await (await Auth.enterPassword).addValue(textConstants.password)
        await (await Auth.signInPasswordButton).click()

        await (await Balance.balance).waitForDisplayed({ timeout: 4000 })
        let balance = await (await Balance.nymBalance).getText()
        //new accounts will always default to mainnet, so 0 balance
        expect(balance).toStrictEqual(textConstants.noNym)
    })
})