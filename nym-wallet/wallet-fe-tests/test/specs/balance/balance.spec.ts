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
    await Helper.elementVisible(Balance.walletAddress);

    let getaccountAddress = await Helper.getAccountAddress();
 
    console.log(getaccountAddress);
    
    await Helper.navigateAndClick(Balance.copyAccountId);
    // disclaimer - I think if it's in clipboard we can use the below...
    // let's try using the clipboard api here - TODO

    // let clipboard = await browser.execute(() => {
    //
    //    }); 
    //  
    // })
  });
});
