import Auth from '../../pageobjects/authScreens';
import Balance from '../../pageobjects/balanceScreen';
const textConstants = require('../../../common/text-constants');
const userData = require('../../../common/user-data.json');
const deleteScript = require('../../../scripts/deletesavedwallet');
const Helper = require('../../../common/helper');
import { newMnemonicLogin } from '../helpers/helper.spec';

// TO-DO figure out how to not repeat steps but also start a fresh test on each run

describe('Create a new account negative scenarios', () => {
  it('generate new mnemonic and verify mnemonic words', () => {
    newMnemonicLogin();

    // // delete an existing saved-wallet.json
    // deleteScript
    // // click through create account flow
    // await Helper.navigateAndClick(Auth.createAccount)
    // // await Helper.elementVisible(Auth.mnemonicPhrase)
    // // save and verify mnemonic
    // const mnemonic = await browser.execute(() => {
    //     return document.getElementById("mnemonicPhrase").innerHTML;
    // });
    // await Helper.mnemonicWordTileIndex(mnemonic)
    // // ensure that once the task above is complete, the 'next' button is enabled
    // const nextButton = await Auth.nextToStep3
    // const isNextDisabled = await nextButton.getAttribute('disabled')
    // expect(isNextDisabled).toBe(null)
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
    await newMnemonicLogin();
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
    await browser.pause(3000);
    await Helper.navigateAndClick(Auth.enterPassword);
    await Helper.addValueToTextField(Auth.enterPassword, textConstants.password);
    await browser.pause(3000);
    await Helper.navigateAndClick(Auth.signInPasswordButton);
    await browser.pause(3000);
    await Helper.elementVisible(Balance.balance);
    //new accounts will always default to mainnet, so 0 balance
    await Helper.verifyStrictText(Balance.nymBalance, textConstants.noNym);
  });
});
