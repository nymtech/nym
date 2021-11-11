const userData = require("../../../common/data/user-data.json");
const helper = require("../../../common/helpers/helper");
const walletLogin = require("../../pages/wallet.login");
const textConstants = require("../../../common/constants/text-constants");
const walletHomepage = require("../../pages/wallet.homepage");
const delegatePage = require("../../pages/wallet.delegate");

describe("delegate to a mix node or gateway", () => {
  it("ensure that fields are enabled for existing user", async () => {
    const mnemonic = await helper.decodeBase(userData.mnemonic);

    await walletLogin.enterMnemonic(mnemonic);

    await helper.navigateAndClick(walletHomepage.delegateButton);

    const getText = await delegatePage.header.getText();

    expect(getText).toEqual(textConstants.delegateHeaderText);
  });

  it("submitting the form without input prompts validation errors", async () => {
    await delegatePage.delegateStakeButton.click();

    const getIdentityValidation =
      await delegatePage.identityValidation.getText();
    const getAmountValidation =
      await delegatePage.amountToDelegateValidation.getText();

    expect(getIdentityValidation).toEqual(
      textConstants.nodeIdentityValidationText
    );
    expect(getAmountValidation).toEqual(textConstants.amountValidationText);
  });

  it("input delegate amount to a mix node then broadcast the transaction then check account balances", async () => {
    const balanceText = await delegatePage.accountBalance.getText();

    const getTransfeeAmount = await delegatePage.transactionFeeAmount.getText();

    await delegatePage.nodeIdentity.setValue(
      userData.identity_key_to_delegate_mix_node
    );

    await delegatePage.amountToDelegate.setValue(userData.delegate_amount);

    //transfer fee + amount delegation
    const sumCost = await helper.calculateFees(
      balanceText,
      getTransfeeAmount,
      userData.delegate_amount,
      false
    );

    await delegatePage.delegateStakeButton.click();

    await delegatePage.successfullyDelegate.waitForClickable({
      timeout: 10000,
    });

    const getConfirmationText =
      await delegatePage.successfullyDelegate.getText();
    expect(getConfirmationText).toContain(textConstants.delegationComplete);

    const availablePunk = await delegatePage.accountBalance.getText();
    //expect new account balance - the fee calculation above

    await delegatePage.finishButton.click();

    expect(await helper.currentBalance(availablePunk)).toEqual(sumCost);
  });

  it("input amount to stake to a gateway then broadcast the transaction then check account balances", async () => {
    const balanceText = await delegatePage.accountBalance.getText();

    const getTransfeeAmount = await delegatePage.transactionFeeAmount.getText();

    await delegatePage.gateWayRadioButton.click();

    await delegatePage.nodeIdentity.setValue(
      userData.identity_key_to_delegate_gateway
    );

    await delegatePage.amountToDelegate.setValue(userData.delegate_amount);

    //transfer fee + amount delegation

    const sumCost = await helper.calculateFees(
      balanceText,
      getTransfeeAmount,
      userData.delegate_amount,
      false
    );

    await delegatePage.delegateStakeButton.click();

    await delegatePage.successfullyDelegate.waitForClickable({
      timeout: 10000,
    });

    const getConfirmationText =
      await delegatePage.successfullyDelegate.getText();
    expect(getConfirmationText).toContain(textConstants.delegationComplete);

    const availablePunk = await delegatePage.accountBalance.getText();
    //expect new account balance - the fee calculation above
    expect(await helper.currentBalance(availablePunk)).toEqual(sumCost);
  });
});
