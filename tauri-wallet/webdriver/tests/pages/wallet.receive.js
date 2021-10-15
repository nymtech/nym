class WalletReceive {

    //can these selectors be nicer like the id's?
    get header() {
        return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardHeader-root > div > span");
    }

    get walletAddress() {
        return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(2) > div > div > div:nth-child(1) > span");
    }

    get copyButton() {
        return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(2) > div > div > div:nth-child(1) > button > span.MuiButton-label");
    }

    get qrCode() {
        return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(2) > div > div > div:nth-child(2) > div > canvas");
    }

    get information() {
        return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(1) > div > div.MuiAlert-message");
    }


    WaitForButtonChangeOnCopy = async () => {
        await this.copyButton.click();

        await this.copyButton.waitForDisplayed({ timeout: 1500 });

        await this.copyButton.waitUntil(async function () {
            return (await this.getText()) === 'COPIED'
        }, {
            timeout: 1500,
            timeoutMsg: 'expected text to be different after 1.5s'
        });
    }

}

module.exports = new WalletReceive()