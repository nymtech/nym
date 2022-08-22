import Auth from '../../pageobjects/authScreens'
import Nav from '../../pageobjects/appNavConstants'
import Balance from '../../pageobjects/balanceScreen'
import Send from '../../pageobjects/sendScreen'
import Receive from '../../pageobjects/receiveScreen'
import Bond from '../../pageobjects/bondScreen'
import Unbond from '../../pageobjects/unbondScreen'
import Delegation from '../../pageobjects/delegationScreen'
const userData = require("../../../common/user-data.json");
const Helper = require('../../../common/helper');


describe('Nav Items behave correctly', () => {

    it('switch from light to dark mode and back', async () => {

        //log in
        await Helper.freshMnemonicLoginQaNetwork()
        // click on different modes
        await Helper.navigateAndClick(Nav.lightMode)
        await Helper.navigateAndClick(Nav.darkMode)
        await Helper.elementVisible(Nav.lightMode)

    })

    it('clicking terminal opens the modal', async () => {

        // ensure the terminal button opens the terminal
        await Helper.elementVisible(Nav.terminalIcon)
        await Helper.navigateAndClick(Nav.terminalIcon)
        await Helper.elementVisible(Nav.terminalTitle)
        await Helper.verifyPartialText(Nav.terminalTitle, 'Terminal')

    })

})

describe('Menu items lead to correct screen', () => {

    //TO-DO none of this works 
    //check each menu item opens the right screen/modal
    it('check Balance link works', async () => {
        await Helper.navigateAndClick(Nav.balance)
        await Helper.verifyPartialText(Balance.balance, 'Balance')
    })

    it('check Send link works', async () => {
        await Helper.navigateAndClick(Nav.send)
        await Helper.verifyPartialText(Send.sendHeader, 'Send')
        await Helper.navigateAndClick(Nav.closeIcon)
    })

    it('check Receive link works', async () => {
        await Helper.navigateAndClick(Nav.receive)
        await Helper.verifyPartialText(Receive.receiveNymTitle, 'Receive NYM')
    })

    it('check Bond link works', async () => {
        await Helper.navigateAndClick(Nav.bond)
        await Helper.verifyPartialText(Bond.bondTitle, 'Bond')
    })

    it('check Unbond link works', async () => {
        await Helper.navigateAndClick(Nav.unbond)
        await Helper.verifyPartialText(Unbond.unbondTitle, 'Unbond')
    })

    it('check Delegation link works', async () => {
        await Helper.navigateAndClick(Nav.delegation)
        await Helper.verifyPartialText(Delegation.delegationTitle, 'Delegation')
    })

})