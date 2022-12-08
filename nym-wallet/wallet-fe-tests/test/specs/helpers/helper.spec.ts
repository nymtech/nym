import Auth from '../../pageobjects/authScreens';

describe('Create a new account and verify login', () => {
  it('generate new mnemonic and verify mnemonic words', async () => {
    // test to check new mnemonic creation
    // will refine shortly
    await browser.pause(1500);

    await Auth.newMnemonicCreation();
  });
});
