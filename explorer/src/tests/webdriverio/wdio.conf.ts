import url = require("./helpers/urls");

export const config: WebdriverIO.Config = {
  specs: ["./tests/specs/*.ts"],
  // Patterns to exclude.
  exclude: [
    // 'path/to/excluded/files'
  ],
  maxInstances: 1,
  capabilities: [
    {
      maxInstances: 5,
      //
      browserName: "chrome",
      acceptInsecureCerts: true,
    },
  ],
  logLevel: "info",
  bail: 0,
  baseUrl: url[process.env.NODE_ENV],
  waitforTimeout: 10000,
  connectionRetryTimeout: 120000,
  connectionRetryCount: 3,
  services: ["chromedriver"],
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: {
    ui: "bdd",
    timeout: 60000,
  },
  autoCompileOpts: {
    autoCompile: true,
    tsNodeOpts: {
      transpileOnly: true,
      project: "tsconfig.json",
    },
  },
};
