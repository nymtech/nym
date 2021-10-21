class WalletCreate {

    get createAccount() {return $('#root > div > div:nth-child(2) > div > div > div:nth-child(2) > button.MuiButtonBase-root.MuiButton-root.MuiButton-contained.MuiButton-containedPrimary.MuiButton-disableElevation.MuiButton-fullWidth > span.MuiButton-label')}

    get accountCreatedSuccessfully() { return $('#root > div > div:nth-child(2) > div > div > div.MuiGrid-root.MuiGrid-container.MuiGrid-item.MuiGrid-justify-content-xs-center > div:nth-child(1) > p')}

    get walletMnemonicCreated() {return $('#root > div > div:nth-child(2) > div > p')}

    get walletMnemonicValue(){ return $('#root > div > div:nth-child(2) > div > div > div.MuiGrid-root.MuiGrid-container.MuiGrid-item.MuiGrid-justify-content-xs-center > div.MuiPaper-root.MuiCard-root.MuiPaper-outlined.MuiPaper-rounded > div > div:nth-child(2) > p')}

    get punkAddress(){ return $('#root > div > div:nth-child(2) > div > div > div.MuiGrid-root.MuiGrid-container.MuiGrid-item.MuiGrid-justify-content-xs-center > div.MuiPaper-root.MuiCard-root.MuiPaper-outlined.MuiPaper-rounded > div > div:nth-child(5) > p')}
    
    get backToSignIn() {return $('#root > div > div:nth-child(2) > div > div > div:nth-child(2) > button > span.MuiButton-label')}

    get signInButton() {return $('[data-testid="sign-in-button"]')}


}
module.exports = new WalletCreate()