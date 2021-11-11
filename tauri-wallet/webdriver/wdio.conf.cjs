const os = require("os");
const path = require("path");
const { spawn, spawnSync } = require("child_process");
//insert path to binary
const nym_path = "../target/release/nym-wallet";

exports.config = {
  //run sequentially, as using one default user may cause issues for parallel test runs for now
  specs: [
    "./tests/specs/existinguser/test.wallet.home.js",
    "./tests/specs/existinguser/test.wallet.send.js",
    "./tests/specs/existinguser/test.wallet.receive.js",
    "./tests/specs/existinguser/test.wallet.bond.js",
    "./tests/specs/existinguser/test.wallet.delegate.js",
    "./tests/specs/newuser/test.wallet.create.js",
  ],

  //run tests by providing --suite {{login}}
  suites: {
    home: ["./tests/specs/existinguser/test.wallet.home.js"],
    sendreceive: [
      "./tests/specs/existinguser/test.wallet.send.js",
      "./tests/specs/existinguser/test.wallet.receive.js",
    ],
    bond: ["./tests/specs/existinguser/test.wallet.bond.js"],
    delegate: [
      "./tests/specs/existinguser/test.wallet.delegate.js",
      "./tests/specs/existinguser/test.wallet.undelegate.js",
    ],
    newuser: ["./tests/specs/newuser/test.wallet.create.js"],
  },
  maxInstances: 1,
  capabilities: [
    {
      maxInstances: 1,
      "tauri:options": {
        application: nym_path,
      },
    },
  ],
  // ===================
  // Test Configurations
  // ===================
  // Define all options that are relevant for the WebdriverIO instance here
  // Level of logging verbosity: trace | debug | info | warn | error | silent
  bail: 0,
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: {
    ui: "bdd",
    timeout: 60000,
  },
  logLevel: "silent",

  // ===================
  // Test Reporters
  // ===================
  reporters: [
    [
      "allure",
      {
        outputDir: "allure-results",
        disableWebdriverStepsReporting: true,
        disableWebdriverScreenshotsReporting: true,
      },
    ],
  ],

  // this is documentented in the readme - you will need to build the project first
  // ensure the rust project is built since we expect this binary to exist for the webdriver sessions
  //onPrepare: () => spawnSync("cargo", ["build", "--release"]),

  // ensure we are running `tauri-driver` before the session starts so that we can proxy the webdriver requests
  beforeSession: () =>
    (tauriDriver = spawn(
      path.resolve(os.homedir(), ".cargo", "bin", "tauri-driver"),
      [],
      { stdio: [null, process.stdout, process.stderr] }
    )),

  afterTest: function (
    test,
    context,
    { error, result, duration, passed, retries }
  ) {
    if (error) {
      browser.takeScreenshot();
    }
  },

  // clean up the `tauri-driver` process we spawned at the start of the session
  afterSession: () => tauriDriver.kill(),
};
