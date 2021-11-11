const userData = require("../../../common/data/user-data.json");
const helper = require("../../../common/helpers/helper");
const textConstants = require("../../../common/constants/text-constants");
const walletLogin = require("../../pages/wallet.login");
const sendWallet = require("../../pages/wallet.send");
const walletHomepage = require("../../pages/wallet.homepage");

describe("send punk to another a wallet", () => {
  it("expect send screen to display the data", async () => {
    const mnemonic = await helper.decodeBase(userData.mnemonic);

    await walletLogin.enterMnemonic(mnemonic);

    await helper.navigateAndClick(walletHomepage.sendButton);

    const textHeader = await sendWallet.sendHeader.getText();

    expect(textHeader).toContain(textConstants.sendPunk);
  });

  it("send funds correctly to another punk address", async () => {
    //already logged in due to the previous test
    const getCurrentBalance = await walletHomepage.accountBalance.getText();

    await sendWallet.toAddress.addValue(userData.receiver_address);

    await sendWallet.amount.addValue(userData.amount_to_send);

    await sendWallet.nextButton.waitForEnabled({ timeout: 3000 });

    await sendWallet.nextButton.click();

    const transFee = await sendWallet.transferFeeAmount.getText();

    await sendWallet.sendButton.click();

    await sendWallet.finishButton.waitForClickable({ timeout: 10000 });

    let sumCost = await helper.calculateFees(
      getCurrentBalance,
      transFee,
      userData.amount_to_send,
      true
    );

    await walletHomepage.accountBalance.isDisplayed();

    const availablePunk = await walletHomepage.accountBalance.getText();

    await sendWallet.finishButton.click();

    //expect new account balance - the fee calculation above
    expect(await helper.currentBalance(availablePunk)).toEqual(sumCost);
  });
});
