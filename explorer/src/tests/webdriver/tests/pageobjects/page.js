var config = require('../../wdio.conf').config

export default class Page {

    open(path) {
        if (path == undefined) {
            return browser.url(config.baseUrl)
        }
        return browser.url(`${config.baseUrl}/${path}`)
    }
}
