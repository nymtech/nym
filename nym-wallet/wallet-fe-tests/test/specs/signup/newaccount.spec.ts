import Auth from '../../pageobjects/authScreens';
import Balance from '../../pageobjects/balanceScreen';
const textConstants = require('../../../common/text-constants');
const userData = require('../../../common/user-data.json');
const deleteScript = require('../../../scripts/deletesavedwallet');
const Helper = require('../../../common/helper');

// TO-DO figure out how to not repeat steps but also start a fresh test on each run

describe('Create a new account negative scenarios', () => {
  it('generate new mnemonic and verify mnemonic words', async () => {
    //create the new mnemoinc
    await Auth.newMnemonicCreation();
  });

  it('click skip password', async () => {
    // click on skip password creation
    await Helper.navigateAndClick(Auth.nextToStep3);
    await Helper.navigateAndClick(Auth.skipPasswordAndSignInWithMnemonic);
    // can see mnemonic login page
    await Helper.elementVisible(Auth.mnemonicInput);
    await Helper.navigateAndClick(Auth.backToSignInOptions);
  });

  it('set up invalid password for new account', async () => {
    // enter invalid password in both fields
    await Helper.navigateAndClick(Auth.password);
    await Helper.addValueToTextField(Auth.password, textConstants.incorrectPassword);
    await Helper.navigateAndClick(Auth.confirmPassword);
    await Helper.addValueToTextField(Auth.confirmPassword, textConstants.incorrectPassword);
    // verify that the 'next' button is still disabled
    const nextButton = await Auth.nextStorePassword;
    const isNextDisabled = await nextButton.getAttribute('disabled');
    expect(isNextDisabled).toBe('true');

    await browser.reloadSession();
  });
});

describe.skip('Create a new account and verify login', () => {
  it('generate new mnemonic and verify mnemonic words', async () => {
    await Auth.newMnemonicCreation();
  });

  it('set up valid password for new account', async () => {
    // enter a valid password in both fields
    await Helper.navigateAndClick(Auth.password);
    await Helper.addValueToTextField(Auth.password, textConstants.password);
    await browser.pause(3000);
    await Helper.navigateAndClick(Auth.confirmPassword);
    await Helper.addValueToTextField(Auth.confirmPassword, textConstants.password);
    await browser.pause(3000);
    // verify that the 'next' button is clickable
    const nextButton = await Auth.nextStorePassword;
    const isNextDisabled = await nextButton.getAttribute('disabled');
    expect(isNextDisabled).toBe(null);
  });

  it('proceed to login with newly created password', async () => {
    // login with a password
    await Helper.navigateAndClick(Auth.nextStorePassword);

    await Helper.navigateAndClick(Auth.enterPassword);
    await Helper.addValueToTextField(Auth.enterPassword, textConstants.password);

    await Helper.navigateAndClick(Auth.signInPasswordButton);

    await Helper.elementVisible(Balance.balance);
    //new accounts will always default to mainnet, so 0 balance
    await Helper.verifyStrictText(Balance.nymBalance, textConstants.noNym);
  });
});
