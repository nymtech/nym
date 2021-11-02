import overviewPage from '../pageobjects/overview.page'
import actionhelper from '../../helpers/actionhelper'

//const baseUrl = 'https://feature-network-explorer-react.ci.nymte.ch/'
//sort all these urls into configuration 
const localHost = 'http://localhost:3000'
const mixnodeUrl = `${localHost}/network-components/mixnodes`
const gatewayUrl = `${localHost}/network-components/gateways`
const blockExplorerUrl = 'https://testnet-milhon-blocks.nymtech.net/validators'


//this is still wip 
//define sequential text execution
describe('Access the overview of the nym explorer', () => {
    it('should match the url from the base configuration', async () => {

        await overviewPage.open()

        await actionhelper.waitUntilPageLoads()

        const getUrl = await browser.getUrl()

        expect(getUrl).toEqual(`${localHost}/overview`)
    })

    it('selecting mixnodes opens the mixnode page', async () => {
        
        await overviewPage.selectMixnode()

        await actionhelper.waitUntilPageLoads()

        const getUrl = await browser.getUrl()

        expect(getUrl).toEqual(mixnodeUrl)

        await browser.back()
    })

    it('selecting gateways opens the gateways page', async () => {
        await overviewPage.selectGateways()

        await actionhelper.waitUntilPageLoads()

        const getUrl = await browser.getUrl()

        expect(getUrl).toEqual(gatewayUrl)

        await browser.back()
    })

    it('selecting validators opens the block explorer', async () => {
        //by selecting validators it opens up the block explorer
        await actionhelper.openSwitchToNewTab(overviewPage.validatorLink)

        const getUrl = await browser.getUrl()

        expect(getUrl).toEqual(blockExplorerUrl)
    })
})


