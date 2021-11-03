
const basicConfig = require('./wdio.conf')

exports.config = {
  ...basicConfig.config,
  // We only need to override the Chrome configuration of capabilities
  capabilities: [
    {
      maxInstances: 5,
      browserName: 'chrome',
      acceptInsecureCerts: true,
      // We need to extends some Chrome flags in order to tell Chrome to run headless
      'goog:chromeOptions': {
        args: ['--headless', '--disable-gpu', '--disable-dev-shm-usage'],
      },
    },
    {
      maxInstances: 5,
      browserName: 'firefox',
      acceptInsecureCerts: true,
      'moz:firefoxOptions': {
        args: ['-headless'],
      },
    },
  ],
}