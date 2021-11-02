//currently targetting feature branch
const baseUrl = "https://feature-network-explorer-react.ci.nymte.ch"

export default class Page {

    open(path) {
        if (path == undefined) {
            return browser.url(baseUrl)
        }
        return browser.url(`${baseUrl}/${path}`)
    }
}
