class WallentUndelegate {
  //todo
  get transactionFee() {
    return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(1) > div > div.MuiGrid-root.MuiGrid-container.MuiGrid-item.MuiGrid-justify-content-xs-space-between.MuiGrid-grid-xs-12 > div:nth-child(2) > div > div.MuiAlert-message");
  }
  get mixNodeRadioButton() {
    return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(1) > div > div.MuiGrid-root.MuiGrid-container.MuiGrid-item.MuiGrid-justify-content-xs-space-between.MuiGrid-grid-xs-12 > div:nth-child(1) > fieldset > div > label:nth-child(1) > span.MuiButtonBase-root.MuiIconButton-root.PrivateSwitchBase-root-20.MuiRadio-root.MuiRadio-colorSecondary.PrivateSwitchBase-checked-21.Mui-checked.MuiIconButton-colorSecondary > span.MuiIconButton-label > input");
  }

  get gatewayRadionButton() {
    return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(1) > div > div.MuiGrid-root.MuiGrid-container.MuiGrid-item.MuiGrid-justify-content-xs-space-between.MuiGrid-grid-xs-12 > div:nth-child(1) > fieldset > div > label:nth-child(2) > span.MuiButtonBase-root.MuiIconButton-root.PrivateSwitchBase-root-20.MuiRadio-root.MuiRadio-colorSecondary.MuiIconButton-colorSecondary > span.MuiIconButton-label > input");
  }

  get nodeIdentity() {
    return $("#mui-55011");
  }

  get stakeButtonConfirmation() {
    return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(2) > button > span.MuiButton-label");
  }
}

module.exports = new WallentUndelegate()