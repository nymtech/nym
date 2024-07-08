const walletLogin = require("../../pages/wallet.login");
const walletSignUp = require("../../pages/wallet.create");
const textConstants = require("../../../common/constants/text-constants");

describe("non existing wallet holder", () => {
  //wallet mnemonic gets pushed here
  const DATA = [];
  it("create a new account and wallet", async () => {
    const signInText = await walletLogin.signInLabel.getText();
    expect(signInText).toEqual(textConstants.homePageSignIn);

    await walletSignUp.createAccount.click();

    //wallet generation takes some time - apply wait
    await walletSignUp.create.click();

    await walletSignUp.accountCreatedSuccessfully.waitForEnabled({
      timeout: 10000,
    });

    const getWalletText = await walletSignUp.punkAddress.getText();
    expect(getWalletText.length).toEqual(43);

    const accountCreated =
      await walletSignUp.accountCreatedSuccessfully.getText();
    expect(accountCreated).toEqual(textConstants.walletSuccess);

    const getMnemonic = await walletSignUp.walletMnemonicValue.getText();
    DATA.push(getMnemonic);
  });

  it("navigate back to sign in screen and validate mnemonic works", async () => {
    await walletSignUp.backToSignIn.click();

    await walletLogin.enterMnemonic(DATA[0]);

    await walletLogin.walletAddress.isDisplayed();
  });
});
