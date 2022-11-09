import Balance from '../../pageobjects/balanceScreen'
import Auth from '../../pageobjects/authScreens'
import Nav from '../../pageobjects/appNavConstants'
import Delegation from '../../pageobjects/delegationScreen'
import Send from '../../pageobjects/sendScreen'
const Helper = require('../../../common/helper');
const textConstants = require("../../../common/text-constants");
const userData = require("../../../common/user-data.json");

describe('Delegate to a mixnode', () => {

    it('entering an invalid node identity key', async () => {

        //login and navigate to the screen
        await Helper.freshMnemonicLoginQaNetwork()
        await Helper.navigateAndClick(Nav.delegation)
        await Helper.elementVisible(Delegation.delegationTitle)
        // TO-DO enter an invalid node
        
    })
})