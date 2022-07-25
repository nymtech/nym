import Auth from '../../pageobjects/authScreens'
import Nav from '../../pageobjects/appNavConstants'
import Balance from '../../pageobjects/balanceScreen'
import Send from '../../pageobjects/sendScreen'
import Receive from '../../pageobjects/receiveScreen'
import Bond from '../../pageobjects/bondScreen'
import Unbond from '../../pageobjects/unbondScreen'
import Delegation from '../../pageobjects/delegationScreen'

const userData = require("../../../common/user-data.json");

describe('Nav Items behave correctly', () => {

    it('switch from light to dark mode and back', async () => {
        //log in
        await Auth.loginWithMnemonic(userData.mnemonic)
        // click on different modes
        await (await Nav.lightMode).waitForDisplayed({ timeout: 4000 })
        await (await Nav.lightMode).click()
        await (await Nav.darkMode).click()
        await (await Nav.lightMode).waitForDisplayed({ timeout: 2500 })
    })

    it('clicking terminal opens the modal', async () => {
        await (await Nav.terminalIcon).click()
        let terminalTitle = await (await Nav.terminalTitle).getText()
        expect(terminalTitle).toContain('Terminal')

    })

})

describe('Menu items lead to correct screen', () => {

    it('check Balance link works', async () => {
        await (await Nav.balance).click()
        let balanceTitle = await (await Balance.balance).getText()
        expect(balanceTitle).toContain('Balance')
    })

    it('check Send link works', async () => {
        await (await Nav.send).click()
        let sendTitle = await (await Send.sendHeader).getText()
        expect(sendTitle).toContain('Send')
        await (await Nav.closeIcon).click()
    })

    it('check Receive link works', async () => {
        await (await Nav.receive).click()
        let receiveNymTitle = await (await Receive.receiveNymTitle).getText()
        expect(receiveNymTitle).toContain('Receive NYM')
    })

    it('check Bond link works', async () => {
        await (await Nav.bond).click()
        let bondTitle = await (await Bond.bondTitle).getText()
        expect(bondTitle).toContain('Bond')
    })

    it('check Unbond link works', async () => {
        await (await Nav.unbond).click()
        let unbondTitle = await (await Unbond.unbondTitle).getText()
        expect(unbondTitle).toContain('Unbond')
    })

    it('check Delegation link works', async () => {
        await (await Nav.delegation).click()
        let delegationTitle = await (await Delegation.delegationTitle).getText()
        expect(delegationTitle).toContain('Delegation')
    })

})