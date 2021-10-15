class WalletLogin {

    get signInLabel() {
        return $("#root > div > div:nth-child(2) > div > h4");
    }

    get mnemonic() {
        return $("#mnemonic");
    }

    get signInButton() {
        return $("#root > div > div:nth-child(2) > div > form > div > div:nth-child(2) > button > span.MuiButton-label");
    }

    get errorValidation() {
        return $("#root > div > div:nth-child(2) > div > form > div > div:nth-child(3) > div > div.MuiAlert-message");
    }

    get accountBalance() {
        return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardHeader-root > div > span");
    }

    get accountBalanceText() {
        return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div > div > div.MuiAlert-message");
    }

    get walletAddress() {
        return $("#root > div > div:nth-child(1) > div:nth-child(3) > div:nth-child(2) > div > div.MuiCardContent-root > div > p");
    }

    get createNewAccount(){
        return $("#root > div > div:nth-child(2) > div > form > div > div:nth-child(3) > a");
    }

    //login to the application
    enterMnemonic = async(mnemonic) => {
        await this.mnemonic.addValue(mnemonic);
        await this.signInButton.click();
        await this.accountBalance.isExisting();
    }
}
module.exports = new WalletLogin()