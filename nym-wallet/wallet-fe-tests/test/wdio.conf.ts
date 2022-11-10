const os = require('os')
const path = require('path')
const { spawn, spawnSync } = require('child_process')

//insert path to binary
const nym_path = '../target/debug/nym-wallet'

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

  suites: {
        signup: [
          './test/specs/signup/*.ts',
        ],
    login: [
      './test/specs/login/*.ts',
    ],
        balance: [
          './test/specs/balance/*.ts',
        ],
        nav: [
          './test/specs/general/*.ts',
        ],
        send: [
          './test/specs/bond/*.ts',
        ],
        delegation: [
          './test/specs/delegation/*.ts',
        ],
  },

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
  
  // ===================
  // Test Configurations
  // ===================
  
  // Level of logging verbosity: trace | debug | info | warn | error | silent
  logLevel: 'error',
  bail: 0,
  framework: 'mocha',
    // reporters: ['spec'],
  mochaOpts: {
    ui: 'bdd',
    timeout: 60000,
  },

  // Reporting tool and settings 

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

  // Things to run before/after each test session

  // onPrepare: () => {
  // let scriptpath = process.cwd() + "/scripts/killprocess.sh";
  // spawn('bash', [scriptpath]);
  // },


  beforeSession: () => {

    (tauriDriver = spawn(
      path.resolve(os.homedir(), '.cargo', 'bin', 'tauri-driver'),
      [],
      { stdio: [null, process.stdout, process.stderr] }
    ))

  },

  // clean up the `tauri-driver` process we spawned at the start of the session
  afterSession: () =>
    tauriDriver.kill(),
}