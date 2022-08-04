import Balance from '../tests/pageobjects/balanceScreen'
import Auth from '../tests/pageobjects/authScreens'
const userData = require("../common/user-data.json");
const deleteScript = require("../scripts/deletesavedwallet")
const savedWalletScript = require("../scripts/savedwalletexists")


class Helpers {

  // clear wallet data, login, and navigate to QA network 
  freshMnemonicLoginQaNetwork = async () => {
    await deleteScript
    await savedWalletScript
    await Auth.loginWithMnemonic(userData.mnemonic)
    await Balance.selectQa()
  }

  loginMnemonic = async () => {
    await Auth.loginWithMnemonic(userData.mnemonic)
  }

  //helper to decode mnemonic so plain 24 character passphrase isn't in sight albeit it is presented when ruunning the scripts
  // TO-DO figure out what's going on with the decoding bit
  decodeBase = async (input) => {
    var m = Buffer.from(input, "base64").toString();
    return m;
  }

  navigateAndClick = async (element) => {
    await element.waitForClickable({ timeout: 6000 })
    await element.click();
  }

  elementVisible = async (element) => {
    await element.waitForDisplayed({ timeout: 6000 })
  }

  elementClickable = async (element) => {
    await element.toBeClickable({ timeout: 8000 })
  }

  addValueToTextField = async (element, value) => {
    await element.addValue(value)
  }

  verifyStrictText = async (element, expectedText) => {
    let error = await element.getText()
    expect(error).toStrictEqual(expectedText)

  }

  verifyPartialText = async (element, expectedText) => {
    let error = await element.getText()
    expect(error).toContain(expectedText)
  }

  currentBalance = async (value) => {
    return parseFloat(value.split(/\s+/)[0].toString()).toFixed(5)
  }


  calculateFees = async (beforeBalance, transactionFee, amount, isSend) => {
    let fee

    if (isSend) {
      //send transaction
      fee = transactionFee.split(/\s+/)[0]
    } else {
      //delegate transaction
      fee = transactionFee.split(/\s+/)[3]
    }

    const currentBalance = beforeBalance.split(/\s+/)[0]
    console.log("currenttttt 2 ............. = " + currentBalance)
    const castCurrentBalance = parseFloat(currentBalance).toFixed(5)
    console.log("castttt ............. " + castCurrentBalance)
    const transCost = +parseFloat(amount) + +parseFloat(fee).toFixed(5)
    console.log("trans ............." + transCost)

    let sum = +castCurrentBalance - transCost
    return sum.toFixed(5)
  }

}

module.exports = new Helpers();
