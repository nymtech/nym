class Bond {

    get bondTitle() { return $("[data-testid='Bond']") }
    get mixnodeRadio() { return $("[data-testid='mix-node']") }
    get gatewayRadio() { return $("[data-testid='gate-way']") }
    get fundsAlert() { return $("[data-testid='fundsAlert']") }


}
export default new Bond()