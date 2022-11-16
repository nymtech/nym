import Balance from '../../pageobjects/balanceScreen';
import Helper from '../../../common/helper';
const textConstants = require('../../../common/text-constants');

describe('Balance screen displays correctly', () => {
  it('selecting qa network', async () => {
    //log in
    await Helper.loginMnemonic();
    // select QA network
    await Helper.navigateAndClick(Balance.networkDropdown);
    await Helper.navigateAndClick(Balance.networkSelectQa);
    // verifty QA network has been selected properly
    await Helper.verifyStrictText(Balance.networkEnv, textConstants.qaNetwork);
  });

  it('copy the account id', async () => {
    // ensure the account number contains *something*
    await browser.pause(25000);
    await Helper.elementVisible(Balance.walletAddress);

    await browser.pause(15000);
    await Helper.verifyPartialText(Balance.walletAddress[1], '1');
    await Helper.navigateAndClick(Balance.copyAccountId);
    // TO-DO is there a way to verify that the copy worked, aka pasting it somewhere maybe?
    // disclaimer - I think if it's in clipboard we can use the below...
    console.log(await browser.sendKeys(['Shift', 'Insert']));
  });
});
