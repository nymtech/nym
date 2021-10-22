const userData = require('../../../common/data/user-data.json');
const helper = require('../../../common/helpers/helper');
const textConstants = require('../../../common/constants/text-constants');
const walletLogin = require('../../pages/wallet.login');
const sendWallet = require('../../pages/wallet.send');
const walletHomepage = require('../../pages/wallet.homepage');

describe("send punk to another a wallet", () => {
    it("expect send screen to display the data", async () => {

        const mnemonic = await helper.decodeBase(userData.mnemonic)

        await walletLogin.enterMnemonic(mnemonic)

        await helper.navigateAndClick(walletHomepage.sendButton)

        const textHeader = await sendWallet.sendHeader.getText()

        expect(textHeader).toContain(textConstants.sendPunk)

    })

    //continue sequential flow for test
    //be wary about production - be wary of funds allocation factor in gas fees
    it("send funds correctly to another punk address", async () => {

        //already logged in due to the previous test
        await sendWallet.toAddress.addValue(userData.receiver_address)

        await sendWallet.amount.addValue(userData.amount_to_send)

        await sendWallet.nextButton.waitForEnabled({ timeout: 3000 })

        await sendWallet.nextButton.click()

        await sendWallet.sendButton.click()

        await sendWallet.finishButton.waitForClickable({ timeout: 10000 })

        await sendWallet.finishButton.click()

        //todo implement asserts around account balance at the start vs the end
    })
})

