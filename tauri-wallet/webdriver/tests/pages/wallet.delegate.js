class WalletDelegate {

  get header() {
    return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardHeader-root > div > span.MuiTypography-root.MuiCardHeader-subheader.MuiTypography-subtitle1.MuiTypography-colorTextSecondary.MuiTypography-displayBlock")
  }

  get nodeIdentity() {
    return $("#identity");
  }

  get accountBalance() {
    return $("#root > div > div:nth-child(1) > div:nth-child(3) > div:nth-child(1) > div > div.MuiCardContent-root > div > div > h6");
  }

  get amountToDelegate() {
    return $("#amount");
  }

  get identityValidation() {
    return $("#identity-helper-text");
  }

  get amountToDelegateValidation() {
    return $("#amount-helper-text");
  }

  get delegateStakeButton() {
    return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(2) > button > span.MuiButton-label");
  }

  get mixNodeRadioButton() {
    return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(1) > div > div.MuiGrid-root.MuiGrid-container.MuiGrid-item.MuiGrid-justify-content-xs-space-between.MuiGrid-grid-xs-12 > div:nth-child(1) > fieldset > div > label:nth-child(1) > span.MuiButtonBase-root.MuiIconButton-root.PrivateSwitchBase-root-8.MuiRadio-root.MuiRadio-colorSecondary.PrivateSwitchBase-checked-9.Mui-checked.MuiIconButton-colorSecondary > span.MuiIconButton-label > input");
  }

  get gateWayRadioButton() {
    return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(1) > div > div.MuiGrid-root.MuiGrid-container.MuiGrid-item.MuiGrid-justify-content-xs-space-between.MuiGrid-grid-xs-12 > div:nth-child(1) > fieldset > div > label:nth-child(2) > span.MuiButtonBase-root.MuiIconButton-root.PrivateSwitchBase-root-8.MuiRadio-root.MuiRadio-colorSecondary.MuiIconButton-colorSecondary > span.MuiIconButton-label > input");
  }

  get transactionFeeAmount() {
    return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(1) > div > div.MuiGrid-root.MuiGrid-container.MuiGrid-item.MuiGrid-justify-content-xs-space-between.MuiGrid-grid-xs-12 > div:nth-child(2) > div > div.MuiAlert-message");
  }

  get successfullyDelegate() {
    return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div:nth-child(1) > div > div.MuiAlert-message > div");
  }

  get finishSuccessDelegation() {
    return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div:nth-child(2) > button");
  }

  //Undelegate

  get unDelegateHeader(){
    return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardHeader-root > div > span.MuiTypography-root.MuiCardHeader-subheader.MuiTypography-subtitle1.MuiTypography-colorTextSecondary.MuiTypography-displayBlock");
  }

  get unNodeIdentity(){
    return $("#mui-52147");
  }

  get unDelegateFeeText(){
    return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(1) > div > div.MuiGrid-root.MuiGrid-container.MuiGrid-item.MuiGrid-justify-content-xs-space-between.MuiGrid-grid-xs-12 > div:nth-child(2) > div > div.MuiAlert-message");
  }

  get unDelegateGatewayRadioButton(){
    return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(1) > div > div.MuiGrid-root.MuiGrid-container.MuiGrid-item.MuiGrid-justify-content-xs-space-between.MuiGrid-grid-xs-12 > div:nth-child(1) > fieldset > div > label:nth-child(2) > span.MuiButtonBase-root.MuiIconButton-root.PrivateSwitchBase-root-8.MuiRadio-root.MuiRadio-colorSecondary.MuiIconButton-colorSecondary > span.MuiIconButton-label > input");
  }

  get unMixNodeRadioButton(){
    return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(1) > div > div.MuiGrid-root.MuiGrid-container.MuiGrid-item.MuiGrid-justify-content-xs-space-between.MuiGrid-grid-xs-12 > div:nth-child(1) > fieldset > div > label:nth-child(1) > span.MuiButtonBase-root.MuiIconButton-root.PrivateSwitchBase-root-8.MuiRadio-root.MuiRadio-colorSecondary.MuiIconButton-colorSecondary > span.MuiIconButton-label > input");
  }

  get unDelegateButton(){
    return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(2) > button > span.MuiButton-label");
  }

}

module.exports = new WalletDelegate()