class Balance {

    get balance() { return $("[data-testid='Balance']") }
    get checkBalance() { return $("[data-testid='check-balance']") }
    get nymBalance() { return $("[data-testid='nym-balance']") }

    get copyAccountId() { return $("[data-testid='copyIcon']")}

    get accountNumber() { return $("[data-testid='accountNumber']")}

    get networkDropdown() { return $("[data-testid='ArrowDropDownIcon']") }
    get networkEnv() { return $("[data-testid='networkEnv']") }
    get networkSelectQa() { return $("[data-testid='QA']") }


}
export default new Balance()