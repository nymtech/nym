import Page from './page'
import Helper from '../../helpers/actionhelper'

class Overview extends Page {

    get mixnodeLink() { return $("[data-testid='Mixnodes']") }
    get gatewayLink() { return $("[data-testid='Gateways']") }
    get validatorLink() { return $("[data-testid='Validators']") }
    get getBlockExplorer() { return $("[data-testid='Validators']") }
    //change the id on this?
    get getDistrbutionText() { return $("[data-testid='Distribution of nodes around the world']") }

    selectMixnode = async () => {
        await this.mixnodeLink.click()
    }

    selectGateways = async () => {

        await this.gatewayLink.click()
    }

    selectValidators = async () => {

        await this.validatorLink.click()
    }

    openBlockExplorer = async () => {

        await this.getBlockExplorer.click()
    }

    distributionText = async (text) => {
        await Helper.validatePageText(this.getDistrbutionText, text)
    }

    open() {
        return super.open('overview')
    }
}

export default new Overview()
