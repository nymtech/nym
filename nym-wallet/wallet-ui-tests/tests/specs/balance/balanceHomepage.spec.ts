import Balance from '../../pageobjects/balanceScreen'
import Auth from '../../pageobjects/authScreens'
const textConstants = require("../../../common/text-constants");
const userData = require("../../../common/user-data.json");
const Helper = require('../../../common/helper');


describe('Balance screen displays correctly', () => {

  it('selecting qa network', async () => {

    //log in
    await Helper.loginMnemonic()
    // select QA network
    await Helper.navigateAndClick(Balance.networkDropdown)
    await Helper.navigateAndClick(Balance.networkSelectQa)
    // verifty QA network has been selected properly
    await Helper.verifyStrictText(Balance.networkEnv, textConstants.qaNetwork)

  })

  it('copy the account id', async () => {

    // ensure the account number contains *something*   
    await Helper.elementVisible(Balance.accountNumber)
    await Helper.verifyPartialText(Balance.accountNumber[1],'1')
    await Helper.navigateAndClick(Balance.copyAccountId)
    // TO-DO is there a way to verify that the copy worked, aka pasting it somewhere maybe? 
    
  })

})