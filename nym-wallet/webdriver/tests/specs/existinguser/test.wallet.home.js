const userData = require("../../../common/data/user-data.json");
const helper = require("../../../common/helpers/helper");
const walletLogin = require("../../pages/wallet.login");
const homepPage = require("../../pages/wallet.homepage");
const textConstants = require("../../../common/constants/text-constants");

describe("wallet splash screen", () => {
  it("should have the sign in header present", async () => {
    const signInText = await walletLogin.signInLabel.getText();
    expect(signInText).toEqual(textConstants.homePageSignIn);
  });

  it("submitting the sign in button with no input throws a validation error", async () => {
    await walletLogin.signInButton.click();

    const errorResponseText = await walletLogin.errorValidation.getText();
    expect(errorResponseText).toEqual(textConstants.homePageErrorMnemonic);
  });

  //currently the punk_address is not fully displayed on the wallet UI
  //trim the punk address
  it("successfully input mnemonic and log in", async () => {
    const mnemonic = await helper.decodeBase(userData.mnemonic);

    await walletLogin.enterMnemonic(mnemonic);

    await walletLogin.walletAddress.waitForEnabled({ timeout: 5000 });

    const getWalletAddress = await walletLogin.walletAddress.getText();
    //currently 35 characters are displayed along with three ...
    //current hack we can assume this is the correct wallet
    const walletTruncated = userData.punk_address.substring(0, 35);

    expect(walletTruncated + "...").toContain(getWalletAddress);
  });

  it("successfully log out the application", async () => {
    await helper.scrollIntoView(homepPage.logOutButton);

    await homepPage.logOutButton.click();

    await walletLogin.signInLabel.waitForEnabled({ timeout: 1500 });
    expect(await walletLogin.signInLabel.isDisplayed()).toEqual(true);
  });
});
