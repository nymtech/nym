<!--
Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
SPDX-License-Identifier: Apache-2.0
-->

# Nym Wallet Webdriverio testsuite

A webdriverio test suite implementation using tauri driver
with a page object model design. This is to provide quick iterative feedback
on the UI of the nym wallet. Currently, tauri-driver is available to run on Windows and Linux machines.

## Installation prerequisites

- `Yarn`
- `NodeJS >= v16.8.0`
- `Rust & cargo >= v1.56.1`
- `tauri-driver`
- `That you have an existing mnemonic and you can login to the app`
- `Have the details listed below to provide the user-data.json file`

## Key Information

- Please read the instructions on the `nym/tauri-wallet/README.md` in the root of the project on how to build the application
- Please ensure you have the relevant Webdriver kits installed on your machine -

```
linux:
 sudo apt-get install -y webkit2gtk-driver
```

```
windows:
download msedgedriver.exe from https://developer.microsoft.com/en-us/microsoft-edge/tools/webdriver/
```

please visit [Tauri Studio](https://tauri.studio/en/docs/usage/guides/webdriver/introduction), this will specify the additional drivers you need

- The path to run the application is set in the `wdio.conf.js` which lives in the root directory
- Before running the suite you need to build the application and check that the application has
  built successfully, if so, you will have an executable sitting in the target directory in `tauri-wallet/target/*/nym_wallet` (refer to point 1)
- The suite will not be able to detect elements on screen if you select a release build, however you can run tests against a release target

## Installation & usage

- `test excution happens inside /webdriver directory`
- `test data needs to be provided inside the user-data.json`
- `check the wdio.conf.cjs to see the test execution along with the path location of the binary`

```
example:
//mnemonic is a base64 enconded value, which is your 24 character passphrase, these values are for illustration purposes
      {
      "mnemonic" : "dGhpcyBpcyBhIHBhc3NwaHJhc2UK",
      "punk_address" : "punk1f3dzkhmunma5ze5q952daxca6371989189",
      "receiver_address" : "punk1p0ce82jxxglpmutvhq4mdwgcwf4avm5n1821982",
      "amount_to_send" : "1",
      "identity_key_to_delegate_mix_node": "value",
      "identity_key_to_delegate_gateway" : "value",
      "delegate_amount" : "1"
      }
```

- `yarn test:runall` - the first test run will take some time to spin up be patient
- You can run tests individually by passing through the script situated in the package.json for example `yarn test:newuser`

Tests are categorised and run by their pages, they follow a sequential flow, if one test case fails before the next execution it may derail the next test.

//todo improve in near future

## Test reporting

Currently the tests use allure reporting, the configuration can be altered in the `wdio.conf.cjs`. At present it takes snapshots of any failing tests, the test output run can be seen in the allure-results directory
Tests ouput:

- <guid-testuite.xml>
- <guid-attachment.png>

If any tests fail in their test run it will produce the stack trace error along with the test in question

## TODO

_Disclaimer_: Still WIP

Implement error handling/ beforeTest() - validating json file exists with data for test execution

Currently this is dev'd against a Linux based OS, not tested against windows yet.
