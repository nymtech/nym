import Balance from '../../pageobjects/balanceScreen'
import Auth from '../../pageobjects/authScreens'
const textConstants = require("../../../common/text-constants");
const userData = require("../../../common/user-data.json");

describe('Balance screen displays correctly', () => {

  it('selecting qa network', async () => {

    //log in
    await Auth.loginWithMnemonic(userData.mnemonic)

    // select QA network
    await (await Balance.networkDropdown).waitForDisplayed({ timeout: 4000 })
    await (await Balance.networkDropdown).click()
    await (await Balance.networkSelectQa).waitForDisplayed({ timeout: 2500 })
    await (await Balance.networkSelectQa).click()

    // verifty QA network has been selected properly
    let network = await (await Balance.networkEnv).getText()
    expect(network).toStrictEqual(textConstants.qaNetwork)

  })

  it('copy the account id', async () => {
    // ensure the account number contains *something*   
    await (await Balance.accountNumber).waitForDisplayed({ timeout: 1500 })
    let accountnumber = await (await Balance.accountNumber).getText()
    expect(accountnumber[1]).toStrictEqual('1')
    await (await Balance.copyAccountId).waitForClickable({ timeout: 1500 })
    await (await Balance.copyAccountId).click()
    // TO-DO is there a way to verify that the copy worked, aka pasting it somewhere maybe? 
  })

})