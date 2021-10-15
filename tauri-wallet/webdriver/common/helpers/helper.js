class Helpers {

    //helper to decode mnemonic so plain 24 character passphrase isn't in sight albeit it is presented when ruunning the scripts
    //maybe a show passphrase toggle button?
    decodeBase = async (input) => {
        var m = Buffer.from(input, 'base64').toString();
        return m;
    }

    navigateAndClick = async (element) => {
        await element.click();
    }

    scrollIntoView = async (element) => {
        await element.scrollIntoView();
    }


    currentBalance = async (value) => {
        return parseFloat(value.split(/\s+/)[0].toString()).toFixed(5);
    }

    //this can be better - todo (fix)
    calculateFees = async (beforeBalance, transactionFee, amount) => {
        const fee = transactionFee.split(/\s+/)[3].toString();
        const currentBalance = beforeBalance.split(/\s+/)[0].toString();

        //cast existing balance to decimal
        const castCurrentBalance = parseFloat(parseFloat(currentBalance).toFixed(5));

        const transCost = +parseFloat(parseFloat(fee).toFixed(5)) +
            +parseFloat(parseFloat(amount));

        return parseFloat(castCurrentBalance - transCost).toFixed(5);
    }
}

module.exports = new Helpers()