class Balance {

    get balance(): Promise<WebdriverIO.Element> { return $("[data-testid='Balance']") }
    get checkBalance(): Promise<WebdriverIO.Element> { return $("[data-testid='check-balance']") }
    get nymBalance(): Promise<WebdriverIO.Element> { return $("[data-testid='nym-balance']") }

    get copyAccountId(): Promise<WebdriverIO.Element> { return $("[data-testid='copyIcon']") } // TO-DO check

    get accountNumber(): Promise<WebdriverIO.Element> { return $("[data-testid='accountNumber']") } // TO-DO check

    get networkDropdown(): Promise<WebdriverIO.Element> { return $("[data-testid='ArrowDropDownIcon']") }
    get networkEnv(): Promise<WebdriverIO.Element> { return $("[data-testid='networkEnv']") }
    get networkSelectQa(): Promise<WebdriverIO.Element> { return $("[data-testid='QA']") }

    selectQa = async () => {
        await (await this.networkDropdown).waitForDisplayed({ timeout: 4000 })
        await (await this.networkDropdown).click()
        await (await this.networkSelectQa).waitForClickable({ timeout: 4000 })
        await (await this.networkSelectQa).click()
        await (await this.networkEnv).waitForClickable({ timeout: 2000 })
    }
}
export default new Balance()