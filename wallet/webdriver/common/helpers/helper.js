class Helpers {
  //helper to decode mnemonic so plain 24 character passphrase isn't in sight albeit it is presented when ruunning the scripts
  //maybe a show passphrase toggle button?
  decodeBase = async (input) => {
    var m = Buffer.from(input, "base64").toString();
    return m;
  };

  navigateAndClick = async (element) => {
    await element.click();
  };

  scrollIntoView = async (element) => {
    await element.scrollIntoView();
  };

  currentBalance = async (value) => {
    return parseFloat(value.split(/\s+/)[0].toString()).toFixed(5);
  };

  //todo need to improve calculation - WIP
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

    const castCurrentBalance = parseFloat(currentBalance).toFixed(5);
    const transCost = +parseFloat(amount) + +parseFloat(fee).toFixed(5);

    let sum = parseFloat(castCurrentBalance) - parseFloat(transCost);
    return sum.toFixed(5);
  };
}

module.exports = new Helpers();
