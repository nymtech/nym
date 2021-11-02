import overviewPage from '../pageobjects/overview.page'
import actionhelper from '../../helpers/actionhelper'

const url = 'https://feature-network-explorer-react.ci.nymte.ch/overview'

describe('Access the overview of the nym explorer', () => {
    it('should match the url from the base configuration', async () => {

        await overviewPage.open()

        await actionhelper.waitUntilPageLoads()

        const getUrl = await browser.getUrl()

        expect(getUrl).toEqual(url)
    })
})


