const config = require("../../wdio.conf").config;

export default class Page {
  open(path: string): string {
    return browser.url(`${config.baseUrl}/${path}`);
  }
}
