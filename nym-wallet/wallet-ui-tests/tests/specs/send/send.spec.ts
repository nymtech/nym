import Balance from '../../pageobjects/balanceScreen'
import Auth from '../../pageobjects/authScreens'
import Nav from '../../pageobjects/appNavConstants'
import Send from '../../pageobjects/sendScreen'
const Helper = require('../../../common/helper');
const textConstants = require("../../../common/text-constants");
const userData = require("../../../common/user-data.json");

describe.skip('Send modal functions correctly', () => {

    it('entering an invalid recipient address shows error', async () => {

        // sign in with mnemonic and select QA
        await Helper.freshMnemonicLoginQaNetwork()
        // click on send and check modal appears
        await Helper.navigateAndClick(Nav.send)
        await Helper.elementVisible(Send.sendHeader)
        // add an invalid recipient address
        await Helper.addValueToTextField(Send.recipientAddress, textConstants.invalidRecipientAddress)
        // TO-DO -- question: should there not be an error message before clicking on Next to warn that the address is invalid?
    })

    it('entering an valid recipient address with negative amount value shows error', async () => {

        await Helper.navigateAndClick(Send.recipientAddress)
        // TO-DO figure out how to clear a text field before adding new value
        await (Send.recipientAddress).clearValue()
        await Helper.addValueToTextField(Send.recipientAddress, userData.receiver_address)
        await Helper.navigateAndClick(Send.sendAmount)
        await Helper.addValueToTextField(Send.sendAmount, textConstants.negativeAmount)
        //next button is still disabled and error message appears
        const nextButton = await Send.next
        const isNextDisabled = await nextButton.getAttribute('disabled')
        expect(isNextDisabled).toBe("true")

    })

    it('enter a valid recipient and value', async () => {

        // enter valid data
        await Helper.addValueToTextField(Send.recipientAddress, userData.receiver_address)
        const getCurrentBalance = await (await Balance.nymBalance).getText()
        await Helper.addValueToTextField(Send.sendAmount, textConstants.amountToSend)
        // click on next and verify details
        await Helper.navigateAndClick(Send.next)
        const fee = await (await Send.fee).getText()
        await Helper.verifyPartialText(Send.sendDetailsHeader, textConstants.sendDetails)
        await Helper.verifyPartialText(Send.amount, textConstants.confirmedAmount)

        await Helper.navigateAndClick(Send.confirm)
        await Helper.elementVisible(Send.viewOnBlockchain)
        await Helper.elementClickable(Send.done)

        // calculate the transaction and verify it has been correctly executed
        let sumCost = await Helper.calculateFees(getCurrentBalance, fee, textConstants.amountToSend, true)
        const getNewBalance = await Balance.nymBalance.getText()

        await Helper.navigateAndClick(Send.done)
        // TO-DO the following fails with "TypeError: elem[prop] is not a function"
        expect(getNewBalance).toEqual(sumCost)
    })
})