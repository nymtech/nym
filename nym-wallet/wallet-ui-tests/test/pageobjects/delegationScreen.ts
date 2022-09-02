class Delegation {

    get delegationTitle() { return $("[data-testid='Delegation']") }
    get delegateStakeButton() { return $("[data-testid='Delegate stake']") }
    get delegateModalHeader() { return $("[data-testid='Delegate']") }

}

export default new Delegation()