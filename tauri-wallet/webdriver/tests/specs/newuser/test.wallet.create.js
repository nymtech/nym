const walletLogin = require('../../pages/wallet.login');
const textConstants = require('../../../common/constants/text-constants');

describe("non existing wallet holder", () => {
    it("create new account", async () => {
        const signInText = await walletLogin.signInLabel.getText();
        expect(signInText).toEqual(textConstants.homePageSignIn);

        await walletLogin.createNewAccount.click();

    })
});