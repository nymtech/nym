class Balance {

    get balance() { return $("[data-testid='Balance']") }
    get checkBalance() { return $("[data-testid='check-balance']") }
    get nymBalance() { return $("[data-testid='nym-balance']") }

    get copyAccountId() { return $("[data-testid='ContentCopyIcon']")}
    get accountId() { return $("[data-testid='']")}


    get networkDropdown() { return $("[data-testid='ArrowDropDownIcon']") }
    get networkSelectQa() { return $("[data-testid='QA']") }


}
export default new Balance()