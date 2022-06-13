const userData = require("../../../common/data/user-data.json");
const textConstants = require("../../../common/constants/text-constants");
const helper = require("../../../common/helpers/helper");
const walletLogin = require("../../pages/wallet.login");
const receive = require("../../pages/wallet.receive");
const walletHomepage = require("../../pages/wallet.homepage");

describe("provide the relevant information about a user nym wallet address", () => {
  it("should have the receivers address and a qr code present", async () => {
    const mnemonic = await helper.decodeBase(userData.mnemonic);

    await walletLogin.enterMnemonic(mnemonic);

    await helper.navigateAndClick(walletHomepage.receiveButton);

    await receive.receiveNymHeader.waitForDisplayed({ timeout: 1500 });

    await receive.WaitForButtonChangeOnCopy();

    const textHeader = await receive.receiveNymHeader.getText();
    const getInformationText = await receive.receiveNymText.getText();
    const getPunkAddress = await receive.walletAddress.getText();

    expect(getPunkAddress).toEqual(userData.punk_address);
    expect(getInformationText).toEqual(textConstants.recievePageInformation);
    expect(textConstants.receivePageHeaderText).toEqual(textHeader);
  });
});
