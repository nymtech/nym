import Balance from '../test/pageobjects/balanceScreen';
import Auth from '../test/pageobjects/authScreens';
const userData = require('../common/user-data.json');
const deleteScript = require('../scripts/deletesavedwallet');
const savedWalletScript = require('../scripts/deletesavedwallet.ts');

class Helpers {
  // clear wallet data, login, and navigate to QA network
  freshMnemonicLoginQaNetwork = async () => {
    await deleteScript;
    await savedWalletScript;
    // await Auth.loginWithMnemonic(userData.mnemonic)
    await this.loginMnemonic();
    await Balance.selectQa();
  };

  // login with a mnemonic
  loginMnemonic = async () => {
    var decodedmnemonic = this.decodeBase(userData.mnemonic);
    await Auth.loginWithMnemonic(decodedmnemonic);
  };

  // click the mnemonic words by index position

  // TO-DO find the best approach
  mnemonicWordTileIndex = async (browser: WebdriverIO.Browser) => {
    let mnemonic = await browser.execute(() => {
      return document.getElementById('mnemonicPhrase').innerHTML;
    });

    let arrayMnemonic = mnemonic.split(' ');

    await this.navigateAndClick(Auth.copyMnemonic);
    await this.navigateAndClick(Auth.iSavedMnemonic);
    // verify the mnemonic words in the correct order
    let mnemonicWordTiles = await await Auth.mnemonicWordTile;
    let wordTileIndex = await await Auth.wordIndex;

    const wordsArray: any[] = [];

    for (const word of mnemonicWordTiles) {
      const wordText = await word.getText();
      const index = arrayMnemonic.indexOf(wordText);
      wordsArray.push({ word, index });
    }
    for (const index of wordTileIndex) {
      const indexValue = await index.getText();
      const match = wordsArray.find((word) => +word.index === +indexValue - 1);
      if (match) {
        await match.word.click();
      }
    }

    const nextButton = await Auth.nextToStep3;
    const isNextDisabled = await nextButton.getAttribute('disabled');
    expect(isNextDisabled).toBe(null);
    await this.navigateAndClick(Auth.nextToStep3);
  };

  // decode user data file
  decodeBase = (input) => {
    const m = Buffer.from(input, 'base64').toString();
    return m;
  };

  // common actions
  navigateAndClick = async (element) => {
    await element.waitForClickable({ timeout: 6000 });
    await element.click();
  };

  elementVisible = async (element) => {
    await element.waitForDisplayed({ timeout: 6000 });
  };

  elementClickable = async (element) => {
    await element.toBeClickable({ timeout: 8000 });
  };

  addValueToTextField = async (element, value) => {
    await element.addValue(value);
  };

  verifyStrictText = async (element, expectedText) => {
    let error = await element.getText();
    expect(error).toStrictEqual(expectedText);
  };

  verifyPartialText = async (element, expectedText) => {
    let error = await element.getText();
    expect(error).toContain(expectedText);
  };

  // token calculations
  currentBalance = async (value) => {
    return parseFloat(value.split(/\s+/)[0].toString()).toFixed(5);
  };

  calculateFees = async (beforeBalance, transactionFee, amount, isSend) => {
    let fee;

    if (isSend) {
      //send transaction
      fee = transactionFee.split(/\s+/)[0];
    } else {
      //delegate transaction
      fee = transactionFee.split(/\s+/)[3];
    }

    const currentBalance = beforeBalance.split(/\s+/)[0];
    console.log('currenttttt 2 ............. = ' + currentBalance);
    const castCurrentBalance = parseFloat(currentBalance).toFixed(5);
    console.log('castttt ............. ' + castCurrentBalance);
    const transCost = +parseFloat(amount) + +parseFloat(fee).toFixed(5);
    console.log('trans .............' + transCost);

    let sum = +castCurrentBalance - transCost;
    return sum.toFixed(5);
  };
}

module.exports = new Helpers();
