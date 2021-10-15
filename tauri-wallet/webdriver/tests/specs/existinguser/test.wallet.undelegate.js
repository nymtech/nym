const userData = require('../../../common/data/user-data');
const helper = require('../../../common/helpers/helper');
const walletLogin = require('../../pages/wallet.login');
const walletHomepage = require('../../pages/wallet.homepage');
const unDelegatePage = require('../../pages/wallet.delegate');

describe("un-delegate to a mix node or gateway", () => {
    it("ensure that fields are enabled for existing user", async () => {
        
        //we are ensuring that the fields are selectable for undelegation 
        //not proceeding to undelegate a node or gateway

        const mnemonic = await helper.decodeBase(userData.mnemonic);

        await walletLogin.enterMnemonic(mnemonic);

        await helper.scrollIntoView(walletHomepage.unDelegateButton);

        await helper.navigateAndClick(walletHomepage.unDelegateButton);

        await unDelegatePage.unDelegateButton.waitForClickable({ timeout: 3000});

        const button = await unDelegatePage.unDelegateButton.isEnabled();
        expect(button).toEqual(true);

        await unDelegatePage.unDelegateGatewayRadioButton.click();
        const gateWaySelected = await unDelegatePage.unDelegateGatewayRadioButton.isSelected();
        
        expect(gateWaySelected).toEqual(true);

        const mixNodeRadioButton = await unDelegatePage.unMixNodeRadioButton.isSelected();
        expect(mixNodeRadioButton).toEqual(false);
    })
});
