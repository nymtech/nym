class Nav {
  get lightMode(): Promise<WebdriverIO.Element> {
    return $("[data-testid='LightModeOutlinedIcon']");
  }
  get darkMode(): Promise<WebdriverIO.Element> {
    return $("[data-testid='ModeNightOutlinedIcon']");
  }
  get terminalTitle(): Promise<WebdriverIO.Element> {
    return $("[data-testid='terminal-header']");
  }
  get terminalIcon(): Promise<WebdriverIO.Element> {
    return $("[data-testid='TerminalIcon']");
  }

  get balance(): Promise<WebdriverIO.Element> {
    return $("[data-testid='Balance']");
  }
  get send(): Promise<WebdriverIO.Element> {
    return $("[data-testid='Send']");
  }
  get receive(): Promise<WebdriverIO.Element> {
    return $("[data-testid='Receive']");
  }
  get bond(): Promise<WebdriverIO.Element> {
    return $("[data-testid='Bond']");
  }
  get unbond(): Promise<WebdriverIO.Element> {
    return $("[data-testid='Unbond']");
  }
  get delegation(): Promise<WebdriverIO.Element> {
    return $("[data-testid='Delegation']");
  }

  get closeIcon(): Promise<WebdriverIO.Element> {
    return $("[data-testid='CloseIcon']");
  }
}
export default new Nav();
