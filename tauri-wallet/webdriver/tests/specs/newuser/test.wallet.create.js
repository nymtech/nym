const walletLogin = require('../../pages/wallet.login');
const walletSignUp = require('../../pages/wallet.create')
const textConstants = require('../../../common/constants/text-constants');

const WALLET_SUCCESS = "Please store your mnemonic in a safe place. You'll need it to access your wallet"

describe("non existing wallet holder", () => {
    it("create new account", async () => {

        const signInText = await walletLogin.signInLabel.getText();
        expect(signInText).toEqual(textConstants.homePageSignIn);

        await walletLogin.createNewAccount.click();

        await walletSignUp.createAccount.click();

        await walletSignUp.accountCreatedSuccessfully.waitForEnabled({ timeout: 6000 });

        const getWalletText = await walletSignUp.punkAddress.getText()
        expect(getWalletText.length).toEqual(43)

        const getMnemonic = await walletSignUp.walletMnemonicCreated.getText()
        expect(getMnemonic).toEqual(WALLET_SUCCESS)

    })
});