class ActionHelper {
  async scrollTo(element) {
    await element.scrollIntoView(element);
  }

  async waitUntilPageLoads() {
    browser.waitUntil(() =>
      browser.execute(() => document.readyState === "complete")
    ),
      {
        timeout: 60 * 1000, // 60 seconds
        timeoutMsg: "Message on failure",
      };
  }

  async validatePageText(element, expectedText: string) {
    const sectionText = await element.getText();
    expect(sectionText).toEqual(expectedText);
  }

  async validateUrl(expectedUrl: string) {
    const urlText = await browser.getUrl();
    expect(urlText).toEqual(expectedUrl);
  }

  async isElementVisible(element) {
    const isVis = await element.isDisplayed();
    expect(isVis).toEqual(true);
  }

  async waitUntilCickable(element) {
    const isClick = await element.isClickable();
    expect(isClick).toEqual(true);
  }

  async openSwitchToNewTab(element) {
    const parentWindow = await browser.getWindowHandle();
    await element.click();
    const getWindows = await browser.getWindowHandles();

    for (let i = 0; i < getWindows.length; i++) {
      if (getWindows[i] != parentWindow) {
        await browser.switchToWindow(getWindows[i]);
        break;
      }
    }
  }
}

export default new ActionHelper();
