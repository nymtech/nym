class Bond {
  // Bonding

  get bondTitle() {
    return $("[data-testid='Bond']");
  }
  get mixnodeRadio() {
    return $("[data-testid='mix-node']");
  }
  get gatewayRadio() {
    return $("[data-testid='gate-way']");
  }
  get fundsAlert() {
    return $("[data-testid='fundsAlert']");
  }

  // Unbonding

  get unbondTitle() {
    return $("[data-testid='Unbond']");
  }
}
export default new Bond();
