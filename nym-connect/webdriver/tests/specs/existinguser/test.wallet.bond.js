const userData = require("../../../common/data/user-data.json");
const helper = require("../../../common/helpers/helper");
const walletLogin = require("../../pages/wallet.login");
const textConstants = require("../../../common/constants/text-constants");
const walletHomepage = require("../../pages/wallet.homepage");
const bondPage = require("../../pages/wallet.bond");

describe("bonding and unbonding nodes", () => {
  it("should have a node already bonded and validate no input fields are enabled", async () => {
    const mnemonic = await helper.decodeBase(userData.mnemonic);

    await walletLogin.enterMnemonic(mnemonic);

    await helper.navigateAndClick(walletHomepage.bondButton);

    await helper.scrollIntoView(bondPage.selectAdvancedOptions);

    await bondPage.selectAdvancedOptions.click();

    //as bond node is mixed expect all the fields to be disabled
    const getText = await bondPage.header.getText();
    const getIdentity = await bondPage.identityKey.isEnabled();
    const getSphinxKey = await bondPage.sphinxKey.isEnabled();
    const amountToBond = await bondPage.amountToBond.isEnabled();
    const hostInput = await bondPage.hostInput.isEnabled();
    const verlocPort = await bondPage.verlocPort.isEnabled();
    const httpApiPort = await bondPage.httpApiPort.isEnabled();
    const mixPort = await bondPage.mixPort.isEnabled();

    //assert all field are not functional
    expect(getText).toEqual(textConstants.bondNodeHeaderText);
    expect(getIdentity).toEqual(false);
    expect(getSphinxKey).toEqual(false);
    expect(amountToBond).toEqual(false);
    expect(hostInput).toEqual(false);
    expect(verlocPort).toEqual(false);
    expect(httpApiPort).toEqual(false);
    expect(mixPort).toEqual(false);
  });

  it("unbond mix monde screen should be present with the option to unbond", async () => {
    //we do not want to unbond our node, check that elements are selectable
    await helper.scrollIntoView(walletHomepage.unBondButton);
    await helper.navigateAndClick(walletHomepage.unBondButton);

    const getText = await bondPage.header.getText();
    const unbondText = await bondPage.unBondWarning.getText();

    await bondPage.unBondButton.isClickable();
    //assert all field are not functional
    expect(getText).toEqual(textConstants.unbondNodeHeaderText);
    expect(unbondText).toEqual(textConstants.unbondMixNodeText);
  });
});
