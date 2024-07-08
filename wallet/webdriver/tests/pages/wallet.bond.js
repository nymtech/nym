class WalletBond {
  get header() {
    return $(
      "#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardHeader-root > div > span.MuiTypography-root.MuiCardHeader-subheader.MuiTypography-subtitle1.MuiTypography-colorTextSecondary.MuiTypography-displayBlock"
    );
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
    return $("#host");
  }
  get versionInput() {
    return $("version");
  }
  get selectAdvancedOptions() {
    return $("[type='checkbox']");
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
    return $("[data-testid='bond-button']");
  }
  get unBondButton() {
    return $("[data-testid='un-bond']");
  }
  get unBond() {
    return $("[data-testid='bond-noded']");
  }
  get unBondWarning() {
    return $("div.MuiAlert-message");
  }
}

module.exports = new WalletBond();
