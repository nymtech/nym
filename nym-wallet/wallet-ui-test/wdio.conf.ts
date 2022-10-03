const os = require('os')
const path = require('path')
const { spawn, spawnSync } = require('child_process')
//insert path to binary
const nym_path = '../target/debug/nym_wallet'

let tauriDriver: any

exports.config = {
  autoCompileOpts: {
    autoCompile: true,
    tsNodeOpts: {
      transpileOnly: true,
      project: 'test/tsconfig.json',
    },
  },
  specs: ['./test/specs/**/*.ts'],
  // Patterns to exclude.
  exclude: [
    // 'path/to/excluded/files'
  ],
  maxInstances: 1,
  capabilities: [
    {
      maxInstances: 1,
      'tauri:options': {
        application: nym_path,
      },
    },
  ],
  //
  // ===================
  // Test Configurations
  // ===================
  // Define all options that are relevant for the WebdriverIO instance here
  //
  // Level of logging verbosity: trace | debug | info | warn | error | silent
  logLevel: 'info',
  bail: 0,
  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: {
    ui: 'bdd',
    timeout: 60000,
  },
  // ===================
  // Test Reporters
  // ===================
  // reporters: [
  //     [
  //         "allure",
  //         {
  //             outputDir: "allure-results",
  //             disableWebdriverStepsReporting: true,
  //             disableWebdriverScreenshotsReporting: true,
  //         },
  //     ],
  // ],

  // this is documentented in the readme - you will need to build the project first
  // ensure the rust project is built since we expect this binary to exist for the webdriver sessions
  //onPrepare: () => spawnSync("cargo", ["build", "--release"]),

  // ensure we are running `tauri-driver` before the session starts so that we can proxy the webdriver requests
  beforeSession: () =>
    (tauriDriver = spawn(path.resolve(os.homedir(), '.cargo', 'bin', 'tauri-driver'), [], {
      stdio: [null, process.stdout, process.stderr],
    })),

  //   afterTest: function (
  //     test,
  //     context,
  //     { error, result, duration, passed, retries }
  //   ) {
  //     if (error) {
  //       browser.takeScreenshot();
  //     }
  //   },

  // clean up the `tauri-driver` process we spawned at the start of the session
  afterSession: () => tauriDriver.kill(),
}
