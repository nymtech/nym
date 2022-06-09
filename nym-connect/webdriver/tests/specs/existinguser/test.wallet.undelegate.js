const userData = require("../../../common/data/user-data.json");
const helper = require("../../../common/helpers/helper");
const walletLogin = require("../../pages/wallet.login");
const walletHomepage = require("../../pages/wallet.homepage");
const unDelegatePage = require("../../pages/wallet.delegate");

describe("un-delegate a mix node or gateway", () => {
  it("ensure that fields are enabled for existing user", async () => {
    //we are ensuring that the fields are selectable for undelegation
    //not proceeding to undelegate a node or gateway

    const mnemonic = await helper.decodeBase(userData.mnemonic);

    await walletLogin.enterMnemonic(mnemonic);

    await helper.scrollIntoView(walletHomepage.unDelegateButton);

    await helper.navigateAndClick(walletHomepage.unDelegateButton);

    await unDelegatePage.unDelegateButton.waitForClickable({ timeout: 1500 });

    await unDelegatePage.unDelegateButton.isEnabled();

    await unDelegatePage.unDelegateGatewayRadioButton.click();

    await unDelegatePage.unDelegateGatewayRadioButton.isSelected();

    const mixNodeRadioButton =
      await unDelegatePage.unMixNodeRadioButton.isSelected();
    expect(mixNodeRadioButton).toEqual(false);
  });
});
