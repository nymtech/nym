const userData = require('../../../common/data/user-data');
const helper = require('../../../common/helpers/helper');
const walletLogin = require('../../pages/wallet.login');
const homepPage = require('../../pages/wallet.homepage');
const textConstants = require('../../../common/constants/text-constants');

describe("Simple navigational and input tests on the wallet home page", () => {
  it("should have the sign in header", async () => {

    const signInText = await walletLogin.signInLabel.getText();

    expect(signInText).toEqual(textConstants.homePageSignIn);
  });

  it("submitting the sign in button with no input throws a validation error", async () => {

    await walletLogin.signInButton.click();

    const errorResponseText = await walletLogin.errorValidation.getText();

    expect(errorResponseText).toEqual(textConstants.homePageErrorMnemonic);
  });

  //currently the punk_address is not fully displayed on the wallet ui wallet
  it("successfully input a bip39 mnemonic and log in", async () => {

    const mnemonic = await helper.decodeBase(userData.mnemonic);

    await walletLogin.enterMnemonic(mnemonic);

    const getWalletAddress = await walletLogin.walletAddress.getText();

    expect(userData.punk_address).toContain(getWalletAddress);
  
  });

  //log out to homepage
  it("successfully log out the application", async () => {

    await helper.scrollIntoView(homepPage.logOutButton);

    await homepPage.logOutButton.click();

    const isNewAccountPresent = await walletLogin.createNewAccount.getText();

    expect(isNewAccountPresent).toEqual(textConstants.createOne);  
  });
});