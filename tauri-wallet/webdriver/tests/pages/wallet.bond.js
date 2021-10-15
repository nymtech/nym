class WalletBond {

  get header() {
    return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardHeader-root > div > span.MuiTypography-root.MuiCardHeader-subheader.MuiTypography-subtitle1.MuiTypography-colorTextSecondary.MuiTypography-displayBlock");
  }

  get identityKey() {
    return $("#identityKey");
  }

  get sphinxKey() {
    return $("#sphinxKey");
  }

  get amountToBond() {
    return $("#amount");
  }

  get hostInput() {
    return $("#host")
  }

  get versionInput() {
    return $("version");
  }

  get advancedOptions() {
    return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div.MuiFormControl-root.MuiFormControl-fullWidth > div:nth-child(1) > div > div:nth-child(8) > label > span.MuiButtonBase-root.MuiIconButton-root.PrivateSwitchBase-root-13.MuiCheckbox-root.MuiCheckbox-colorSecondary.MuiIconButton-colorSecondary > span.MuiIconButton-label > input");
  }

  get selectAdvancedOptions() {
    return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div.MuiFormControl-root.MuiFormControl-fullWidth > div:nth-child(1) > div > div:nth-child(8) > label > span.MuiButtonBase-root.MuiIconButton-root.PrivateSwitchBase-root-7.MuiCheckbox-root.MuiCheckbox-colorSecondary.MuiIconButton-colorSecondary");

  }

  get mixPort() {
    return $("#mixPort");
  }

  get verlocPort() {
    return $("#verlocPort");
  }

  get httpApiPort() {
    return $("#httpApiPort");
  }

  get bondButton() {
    return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div.MuiFormControl-root.MuiFormControl-fullWidth > div:nth-child(2) > button");
  }

  get unBondButton() {
    return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div.MuiAlert-action > button > span.MuiButton-label");
  }

  get unBondInformation(){
    return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div.MuiAlert-message");
  }

}

module.exports = new WalletBond()