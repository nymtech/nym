class ActionHelper {

    scrollTo = async (element) => {
        await element.scrollIntoView(element);
    }

    waitUntilPageLoads = async () => {
        browser.waitUntil(
            () => browser.execute(() => document.readyState === 'complete')),
        {
            timeout: 60 * 1000, // 60 seconds
            timeoutMsg: 'Message on failure'
        }
    }

    validatePageText = async (element, expectedText) => {
        const sectionText = await element.getText()
        expect(sectionText).toEqual(expectedText)
    }

    validateUrl = async (expectedUrl) => {
        const urlText = await browser.getUrl()
        expect(urlText).toEqual(expectedUrl)
    }

    isElementVisible = async (element) => {
        const isVis = await element.isDisplayed()
        expect(isVis).toEqual(true)
    }
}

export default new ActionHelper()