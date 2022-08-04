class Balance {

    get balance() { return $("[data-testid='Balance']") }
    get checkBalance() { return $("[data-testid='check-balance']") }
    get nymBalance() { return $("[data-testid='nym-balance']") }

    get copyAccountId() { return $("[data-testid='copyIcon']") }

    get accountNumber() { return $("[data-testid='accountNumber']") }

    get networkDropdown() { return $("[data-testid='ArrowDropDownIcon']") }
    get networkEnv() { return $("[data-testid='networkEnv']") }
    get networkSelectQa() { return $("[data-testid='QA']") }

    selectQa = async () => {
        await this.networkDropdown.waitForDisplayed({ timeout: 4000 })
        await this.networkDropdown.click()
        await this.networkSelectQa.waitForClickable({ timeout: 4000 })
        await this.networkSelectQa.click()
        await this.networkEnv.waitForClickable({ timeout: 2000 })
    }
}
export default new Balance()