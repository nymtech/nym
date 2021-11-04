import Page from "./page";
import Helper from "../../helpers/actionhelper";

class Overview extends Page {
  get mixnodeLink(): WebdriverIO.Element {
    return $("[data-testid='Mixnodes']");
  }

  get gatewayLink(): WebdriverIO.Element {
    return $("[data-testid='Gateways']");
  }

  get validatorLink(): WebdriverIO.Element {
    return $("[data-testid='Validators']");
  }

  get getBlockExplorer(): WebdriverIO.Element {
    return $("[data-testid='Validators']");
  }

  get getDistrbutionText(): WebdriverIO.Element {
    return $("[data-testid='Distribution of nodes around the world']");
  }

  async selectMixnode() {
    await this.mixnodeLink.click();
  }

  async selectGateways() {
    await this.gatewayLink.click();
  }

  async selectValidators() {
    await this.validatorLink.click();
  }

  async openBlockExplorer() {
    await this.getBlockExplorer.click();
  }

  async distributionText(text: string) {
    await Helper.validatePageText(this.getDistrbutionText, text);
  }

  open() {
    return super.open("overview");
  }
}

export default new Overview();
