import Page from './page'

class Overview extends Page {

    //there's multiple of these, specify based upon the index
    //0 = mixnode
    //1 = gateways
    //2 = validators
    get arrowForwardIcon () { return $$("[data-testid='ArrowForwardSharpIcon']") }
    
    async selectMixnode () {
        //todo sort the index
        await this.arrowForwardIcon.click()
    }
    
    async selectGateways () {
        //todo sort the index
        await this.arrowForwardIcon.click()
    }

    async selectValidators () {
        //todo sort the index
        await this.arrowForwardIcon.click()
    }

    open () {
        return super.open('overview')
    }
}

export default new Overview()
