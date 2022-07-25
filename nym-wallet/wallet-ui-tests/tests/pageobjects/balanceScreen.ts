class Balance {

    get balance() { return $("[data-testid='Balance']") }
    get checkBalance() { return $("[data-testid='check-balance']") }
    get nymBalance() { return $("[data-testid='nym-balance']") }

     // TO-DO figure out what's wrong with this copyAccountId locator being intercepted when clicked on 
    get copyAccountId() { return $("[data-testid='ContentCopyIcon']")}
    // get copyAccountId() { return $("[data-testid='copyy']")}

    get accountNumber() { return $("[data-testid='accountNumber']")}

    get networkDropdown() { return $("[data-testid='ArrowDropDownIcon']") }
    get networkEnv() { return $("[data-testid='networkEnv']") }
    get networkSelectQa() { return $("[data-testid='QA']") }


}
export default new Balance()