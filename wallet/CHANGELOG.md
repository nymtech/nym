# Changelog

## [Unreleased]

## [v1.2.13] (2024-05-08)

- Bug fix: wallet delegations list is empty when RPC node doesn't hold block ([#4565])
- updating sign commands to include nym-node([#4578])

[#4578]: https://github.com/nymtech/nym/pull/4578
[#4565]: https://github.com/nymtech/nym/pull/4565

## [v1.2.10] (2023-10-31)

- Add loading model on initial load of delegations ([#4039])
- remove any whitespace from input field when bonding host ([#4062])

[#4039]: https://github.com/nymtech/nym/pull/4039
[#4062]: https://github.com/nymtech/nym/pull/4062

## [v1.2.9] (2023-10-10)

- Wallet: Introduce edit account name ([#3895])

[#3895]: https://github.com/nymtech/nym/pull/3895

## [v1.2.8] (2023-08-23)

- [hotfix]: don't assign invalid fields when crossing the JS boundary ([#3805])

[#3805]: https://github.com/nymtech/nym/pull/3805

## [1.2.7] (2023-08-17)
- release due to schema changes in the contract

## [v1.2.6] (2023-07-18)

- [wallet] bugfix: don't send funds for pledge decrease simulation ([#3676])

[#3676]: https://github.com/nymtech/nym/pull/3676

## [v1.2.5] (2023-07-04)

- Wallet - add "Test my node" in the Node Settings and show its results ([#2314])
- Move client wasm to Wallet directory ([#3606])

[#2314]: https://github.com/nymtech/nym/issues/2314
[#3606]: https://github.com/nymtech/nym/pull/3606

## [v1.2.4] (2023-06-06)

- Wallet - Bonding page does not load the node info automatically unless you navigate back and forth from it! ([#3478])

[#3478]: https://github.com/nymtech/nym/issues/3478

## [v1.2.3] (2023-05-16)

- Wallet - Allow users to choose validator under the Advanced Settings 🟢 ([#2489])

[#2489]: https://github.com/nymtech/nym/issues/2489

## [v1.2.2] (2023-05-02)

- Wallet - change "Bond more" action to "Change bond amount" to allow for increasing and decreasing the amount in the same modal ([#2134])
- Split wallet sign in and main into two entry points ([#3188])

[#2134]: https://github.com/nymtech/nym/issues/2134
[#3188]: https://github.com/nymtech/nym/pull/3188

## [v1.2.1] (2023-04-18)

- [bug] bonding signature with vesting account ([#3315])
- Wallet - settings: fix version check ([#3306])
- Wallet - add new tauri command to fetch app version info ([#3305])
- Wallet - backend support to change password in file ([#3285])
- Wallet - add missing tooltips ([#3275])
- Wallet - global settings part 1 general and advanced 🟢 ([#2181])
- Wallet - global settings part 2 Security - Add&Change Password 🟢 ([#2275])
- feat(wallet): app version check (#3308) ([#3319])

[#3315]: https://github.com/nymtech/nym/issues/3315
[#3306]: https://github.com/nymtech/nym/issues/3306
[#3305]: https://github.com/nymtech/nym/issues/3305
[#3285]: https://github.com/nymtech/nym/issues/3285
[#3275]: https://github.com/nymtech/nym/issues/3275
[#2181]: https://github.com/nymtech/nym/issues/2181
[#2275]: https://github.com/nymtech/nym/issues/2275
[#3319]: https://github.com/nymtech/nym/pull/3319

## [v1.2.0] (2023-04-04)

- Add Version and Location to gateway settings ([#3266])
- Wallet - bonding screen - short info with link ([#3179])
- Wallet - discrepancies figma vs wallet ([#2183])
- Wallet - If a gateway is bonded show gateway settings in the node settings: ([#2242])

[#3266]: https://github.com/nymtech/nym/issues/3266
[#3179]: https://github.com/nymtech/nym/issues/3179
[#2183]: https://github.com/nymtech/nym/issues/2183
[#2242]: https://github.com/nymtech/nym/issues/2242

## [v1.1.12] (2023-03-15)

- Wallet - in the vesting section separate the "Unlocked transferable tokens" into "Unlocked vesting tokens" and "Transferable rewards" for better clarity ([#3132])
- Wallet - change the Bonding flow to include an additional step for the user to provide a unique signature when they bond their node ([#3109])

[#3132]: https://github.com/nymtech/nym/issues/3132
[#3109]: https://github.com/nymtech/nym/issues/3109

## [v1.1.11] (2023-03-07)

- Wallet: optional gas and memo fields ([#2222])

[#2222]: https://github.com/nymtech/nym/issues/2222

## [v1.1.10] (2023-02-20)

- Wallet - Fix send address bug ([#3030])
- Wallet - [Issue] the `Operator cost` field in the Playground does not allow values above 100.0000 ([#2998])
- nym-wallet: fix auto-updated files ([#3043])

[#3030]: https://github.com/nymtech/nym/issues/3030
[#2998]: https://github.com/nymtech/nym/issues/2998
[#3043]: https://github.com/nymtech/nym/pull/3043

## [nym-wallet-v1.1.9](https://github.com/nymtech/nym/releases/tag/nym-wallet-v1.1.9) (2023-02-14)

- Allow more flexibility for user when setting passwords ([#2993])
- User feedback on weak passwords ([#2993])
- User no longer has to copy mnemonic to continune account creation ([#2948])
- Updated instructional steps for creating accounts with a password ([#2962])

[#2948]: https://github.com/nymtech/nym/issues/2948
[#2993]: https://github.com/nymtech/nym/issues/2993
[#2962]: https://github.com/nymtech/nym/issues/2962

## [nym-wallet-v1.1.8](https://github.com/nymtech/nym/releases/tag/nym-wallet-v1.1.8) (2023-01-24)

- Fix delegations sorting ([#2885])

[#2885]: https://github.com/nymtech/nym/issues/2885

## [nym-wallet-v1.1.7](https://github.com/nymtech/nym/releases/tag/nym-wallet-v1.1.7) (2023-01-17)

- link to the ng mixnet explorer for account info ([#2823])
- Fix delegations sorting ([#2885]) (2023-01-23)

[#2823]: https://github.com/nymtech/nym/pull/2823

## [nym-wallet-v1.1.6](https://github.com/nymtech/nym/releases/tag/nym-wallet-v1.1.6) (2023-01-10)

- Fix param input layout **1.1.5 Release** by @fmtabbara in https://github.com/nymtech/nym/pull/2720
- Add epoch info to unbond modal **1.1.5 Release** by @fmtabbara in https://github.com/nymtech/nym/pull/2718
- Feat/2130 tables update rebase by @gala1234 https://github.com/nymtech/nym/pull/2742
- pass wallet address as url param by @doums in https://github.com/nymtech/nym/pull/2780

## [nym-wallet-v1.1.5](https://github.com/nymtech/nym/releases/tag/nym-wallet-v1.1.5) (2022-12-22)

This release contains the APY calculator feature.

## [nym-wallet-v1.1.5](https://github.com/nymtech/nym/releases/tag/nym-wallet-v1.1.5) (2022-12-22)

This release contains the APY calculator feature.

## [nym-wallet-v1.1.4](https://github.com/nymtech/nym/releases/tag/nym-wallet-v1.1.4) (2022-12-20)

This release contains a bugfix for hiding and displaying the mnemonic, and displays the Bity wallet address in the buy page, as well as better error handling.

- wallet: present some cases of the more technical errors (abci, ..) in a more human readable form.
- Bug fix/hide display mnemonic by @fmtabbara in https://github.com/nymtech/nym/pull/1874
- adding bity icon and wallet address by @gala1234 in https://github.com/nymtech/nym/pull/2689
- remove warning error message by @gala1234 in https://github.com/nymtech/nym/pull/2694
- ui and copy changes by @gala1234 in https://github.com/nymtech/nym/pull/2691

## [nym-wallet-v1.1.3](https://github.com/nymtech/nym/releases/tag/nym-wallet-v1.1.3) (2022-12-13)

- wallet: improved unbond screen.

## [nym-wallet-v1.1.2](https://github.com/nymtech/nym/releases/tag/nym-wallet-v1.1.2) (2022-11-09)

- wallet: Bity buy functionality

## [nym-wallet-v1.1.1](https://github.com/nymtech/nym/releases/tag/nym-wallet-v1.1.1) (2022-11-09)

- wallet: compatibility with nym-binaries-1.1.1

## [nym-wallet-v1.1.0](https://github.com/nymtech/nym/releases/tag/nym-wallet-v1.1.0) (2022-11-09)

- wallet: Add window for showing logs for when the terminal is not available.
- wallet: Rework of rewarding ([#1472])
- wallet: Highlight delegations on an unbonded node to notify the delegator to undelegate stake
- wallet: Group delegations on unbonded nodes at the top of the list
- wallet: Simplified delegation screen when user has no delegations
- wallet: Update "create password" howto guide
- wallet: Allow setting of operating cost when node is bonded
- wallet: Allow update of operating cost on bonded node
- wallet: New bond settings area
- wallet: Display pending unbonds
- wallet: Display routing score and average score for gateways
- wallet: Improve dialogs for account creation

[#1472]: https://github.com/nymtech/nym/pull/1472

## [nym-wallet-v1.0.9](https://github.com/nymtech/nym/releases/tag/nym-wallet-v1.0.8) (2022-09-08)

- wallet: change default `nyxd` URL to https://rpc.nymtech.net

## [nym-wallet-v1.0.8](https://github.com/nymtech/nym/releases/tag/nym-wallet-v1.0.8) (2022-08-11)

- wallet: new bonding flow and screen for bonded node
- wallet: compound and redeem functionalities for operator rewards
- wallet: a few minor touch ups and bug fixes

## [nym-wallet-v1.0.7](https://github.com/nymtech/nym/tree/nym-wallet-v1.0.7) (2022-07-11)

- wallet: dark mode
- wallet: when simulating gas costs, an automatic adjustment is being used ([#1388]).

[#1388]: https://github.com/nymtech/nym/pull/1388

## [nym-wallet-v1.0.6](https://github.com/nymtech/nym/tree/nym-wallet-v1.0.6) (2022-06-21)

- wallet: undelegating now uses either the mixnet or vesting contract, or both, depending on how delegations were made
- wallet: redeeming and compounding now uses both the mixnet and vesting contract
- wallet: the wallet backend learned how to archive wallet files
- wallet: add ENABLE_QA_MODE environment variable to enable QA mode on built wallet

## [nym-wallet-v1.0.5](https://github.com/nymtech/nym/tree/nym-wallet-v1.0.5) (2022-06-14)

- wallet: add simple CLI tool for decrypting and recovering the wallet file.
- wallet: added support for multiple accounts ([#1265])
- wallet: compound and claim reward endpoints for operators and delegators ([#1302])
- wallet: require password to switch accounts
- wallet: the wallet backend learned how to keep track of validator name, either hardcoded or by querying the status endpoint.
- wallet: new delegation and rewards UI
- wallet: show version in nav bar
- wallet: contract admin route put back
- wallet: staking_supply field to StateParams
- wallet: show transaction hash for redeeming or compounding rewards

[#1265]: https://github.com/nymtech/nym/pull/1265
[#1302]: https://github.com/nymtech/nym/pull/1302

## [nym-wallet-v1.0.4](https://github.com/nymtech/nym/tree/nym-wallet-v1.0.4) (2022-05-04)

### Changed

- all: the default behaviour of validator client is changed to use `broadcast_sync` and poll for transaction inclusion instead of using `broadcast_commit` to deal with timeouts ([#1246])

## [nym-wallet-v1.0.3](https://github.com/nymtech/nym/tree/nym-wallet-v1.0.3) (2022-04-25)

[Full Changelog](https://github.com/nymtech/nym/compare/nym-binaries-1.0.0-rc.2...nym-wallet-v1.0.3)

**Fixed bugs:**

- \[Issue\] Wallet 1.0.2 cannot send NYM tokens from a DelayedVestingAccount [\#1215](https://github.com/nymtech/nym/issues/1215)
- Main README not showing properly with GitHub dark mode [\#1211](https://github.com/nymtech/nym/issues/1211)

**Merged pull requests:**

- Bugfix - wallet undelegation for vesting accounts [\#1220](https://github.com/nymtech/nym/pull/1220) ([mmsinclair](https://github.com/mmsinclair))
- Bugfix/delegation reconcile [\#1219](https://github.com/nymtech/nym/pull/1219) ([jstuczyn](https://github.com/jstuczyn))
- Bugfix/query proxied pending delegations [\#1218](https://github.com/nymtech/nym/pull/1218) ([jstuczyn](https://github.com/jstuczyn))
- Using custom gas multiplier in the wallet [\#1217](https://github.com/nymtech/nym/pull/1217) ([jstuczyn](https://github.com/jstuczyn))
- Feature/vesting accounts support [\#1216](https://github.com/nymtech/nym/pull/1216) ([jstuczyn](https://github.com/jstuczyn))
- Release/1.0.0 rc.2 [\#1214](https://github.com/nymtech/nym/pull/1214) ([jstuczyn](https://github.com/jstuczyn))
- chore: fix dark mode rendering [\#1212](https://github.com/nymtech/nym/pull/1212) ([pwnfoo](https://github.com/pwnfoo))
- Feature/spend coconut [\#1210](https://github.com/nymtech/nym/pull/1210) ([neacsu](https://github.com/neacsu))
- Bugfix/unique sphinx key [\#1207](https://github.com/nymtech/nym/pull/1207) ([jstuczyn](https://github.com/jstuczyn))
- Add cache read and write timeouts [\#1206](https://github.com/nymtech/nym/pull/1206) ([durch](https://github.com/durch))
- Additional, more informative routes [\#1204](https://github.com/nymtech/nym/pull/1204) ([durch](https://github.com/durch))
- Feature/aggregated econ dynamics explorer endpoint [\#1203](https://github.com/nymtech/nym/pull/1203) ([jstuczyn](https://github.com/jstuczyn))
- Debugging validator [\#1198](https://github.com/nymtech/nym/pull/1198) ([durch](https://github.com/durch))
- wallet: expose additional validator configuration functionality to the frontend [\#1195](https://github.com/nymtech/nym/pull/1195) ([octol](https://github.com/octol))
- Update rewarding validator address [\#1193](https://github.com/nymtech/nym/pull/1193) ([durch](https://github.com/durch))
- Crypto part of the Groth's NIDKG [\#1182](https://github.com/nymtech/nym/pull/1182) ([jstuczyn](https://github.com/jstuczyn))
- fix unbond page [\#1180](https://github.com/nymtech/nym/pull/1180) ([tommyv1987](https://github.com/tommyv1987))
- Type safe bounds [\#1179](https://github.com/nymtech/nym/pull/1179) ([durch](https://github.com/durch))
- Fix delegation paging [\#1174](https://github.com/nymtech/nym/pull/1174) ([durch](https://github.com/durch))
- Update binaries to rc version [\#1172](https://github.com/nymtech/nym/pull/1172) ([tommyv1987](https://github.com/tommyv1987))
- Bump ansi-regex from 4.1.0 to 4.1.1 in /docker/typescript_client/upload_contract [\#1171](https://github.com/nymtech/nym/pull/1171) ([dependabot[bot]](https://github.com/apps/dependabot))

## [nym-wallet-v1.0.2](https://github.com/nymtech/nym/tree/nym-wallet-v1.0.2) (2022-04-05)

[Full Changelog](https://github.com/nymtech/nym/compare/nym-wallet-v1.0.1...nym-wallet-v1.0.2)

**Merged pull requests:**

- Wallet 1.0.2 visual tweaks [\#1197](https://github.com/nymtech/nym/pull/1197) ([mmsinclair](https://github.com/mmsinclair))
- Password for wallet with routes [\#1196](https://github.com/nymtech/nym/pull/1196) ([fmtabbara](https://github.com/fmtabbara))
- Add auto-updater to Nym Wallet [\#1194](https://github.com/nymtech/nym/pull/1194) ([mmsinclair](https://github.com/mmsinclair))
- Fix clippy warnings for beta toolchain [\#1191](https://github.com/nymtech/nym/pull/1191) ([octol](https://github.com/octol))
- wallet: expose validator urls to the frontend [\#1190](https://github.com/nymtech/nym/pull/1190) ([octol](https://github.com/octol))
- wallet: add test for decrypting stored wallet file [\#1189](https://github.com/nymtech/nym/pull/1189) ([octol](https://github.com/octol))
- Fix clippy warnings [\#1188](https://github.com/nymtech/nym/pull/1188) ([octol](https://github.com/octol))
- Password for wallet with routes [\#1187](https://github.com/nymtech/nym/pull/1187) ([mmsinclair](https://github.com/mmsinclair))
- wallet: add validate_mnemonic [\#1186](https://github.com/nymtech/nym/pull/1186) ([octol](https://github.com/octol))
- wallet: support removing accounts from the wallet file [\#1185](https://github.com/nymtech/nym/pull/1185) ([octol](https://github.com/octol))
- Feature/adding discord [\#1184](https://github.com/nymtech/nym/pull/1184) ([gala1234](https://github.com/gala1234))
- wallet: config backend for validator selection [\#1183](https://github.com/nymtech/nym/pull/1183) ([octol](https://github.com/octol))
- Add storybook to wallet [\#1178](https://github.com/nymtech/nym/pull/1178) ([mmsinclair](https://github.com/mmsinclair))
- wallet: connection test nyxd and api urls independently [\#1170](https://github.com/nymtech/nym/pull/1170) ([octol](https://github.com/octol))
- wallet: wire up account storage [\#1153](https://github.com/nymtech/nym/pull/1153) ([octol](https://github.com/octol))
- Feature/signature on deposit [\#1151](https://github.com/nymtech/nym/pull/1151) ([neacsu](https://github.com/neacsu))

## [nym-wallet-v1.0.1](https://github.com/nymtech/nym/tree/nym-wallet-v1.0.1) (2022-04-05)

[Full Changelog](https://github.com/nymtech/nym/compare/nym-binaries-1.0.0-rc.1...nym-wallet-v1.0.1)

**Closed issues:**

- Check enabling bbbc simultaneously with open access. Estimate what it would take to make this the default compilation target. [\#1175](https://github.com/nymtech/nym/issues/1175)
- Get coconut credential for deposited tokens [\#1138](https://github.com/nymtech/nym/issues/1138)
- Make payments lazy [\#1135](https://github.com/nymtech/nym/issues/1135)
- Uptime on node selection for sets [\#1049](https://github.com/nymtech/nym/issues/1049)

## [nym-wallet-v1.0.0](https://github.com/nymtech/nym/tree/nym-wallet-v1.0.0) (2022-02-03)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.12.1...nym-wallet-v1.0.0)

**Implemented enhancements:**

- \[Feature Request\] Please enable registration without need for Telegram account [\#1016](https://github.com/nymtech/nym/issues/1016)
- Fast mixnode launch with a pre-built ISO + VM software [\#1001](https://github.com/nymtech/nym/issues/1001)

**Fixed bugs:**

- \[Issue\] [\#1000](https://github.com/nymtech/nym/issues/1000)
- \[Issue\] `nym-client` requires multiple attempts to run a server [\#869](https://github.com/nymtech/nym/issues/869)
- De-'float'-ing `Interval` \(`Display` impl + `serde`\) [\#1065](https://github.com/nymtech/nym/pull/1065) ([jstuczyn](https://github.com/jstuczyn))
- display client address on wallet creation [\#1058](https://github.com/nymtech/nym/pull/1058) ([fmtabbara](https://github.com/fmtabbara))

**Closed issues:**

- Rewarded set inclusion probability API endpoint [\#1037](https://github.com/nymtech/nym/issues/1037)
- Update cw-storage-plus to 0.11 [\#1032](https://github.com/nymtech/nym/issues/1032)
- Change `u128` fields in `RewardEstimationResponse` to `u64` [\#1029](https://github.com/nymtech/nym/issues/1029)
- Test out the mainnet Gravity Bridge [\#1006](https://github.com/nymtech/nym/issues/1006)
- Add vesting contract interface to nym-wallet [\#959](https://github.com/nymtech/nym/issues/959)
- Mixnode crash [\#486](https://github.com/nymtech/nym/issues/486)

**Merged pull requests:**

- create custom urls for mainnet [\#1095](https://github.com/nymtech/nym/pull/1095) ([fmtabbara](https://github.com/fmtabbara))
- Wallet signing on MacOS [\#1093](https://github.com/nymtech/nym/pull/1093) ([mmsinclair](https://github.com/mmsinclair))
- Fix rust 2018 idioms warnings [\#1092](https://github.com/nymtech/nym/pull/1092) ([octol](https://github.com/octol))
- Prevent contract overwriting [\#1090](https://github.com/nymtech/nym/pull/1090) ([durch](https://github.com/durch))
- Logout operation [\#1087](https://github.com/nymtech/nym/pull/1087) ([jstuczyn](https://github.com/jstuczyn))
- Update to rust edition 2021 everywhere [\#1086](https://github.com/nymtech/nym/pull/1086) ([octol](https://github.com/octol))
- Tag contract errors, and print out lines for easier QA [\#1084](https://github.com/nymtech/nym/pull/1084) ([durch](https://github.com/durch))
- Feature/flexible vesting + utility queries [\#1083](https://github.com/nymtech/nym/pull/1083) ([durch](https://github.com/durch))
- Bump @openzeppelin/contracts from 4.3.1 to 4.4.2 in /contracts/basic-bandwidth-generation [\#1082](https://github.com/nymtech/nym/pull/1082) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump nth-check from 2.0.0 to 2.0.1 in /clients/native/examples/js-examples/websocket [\#1081](https://github.com/nymtech/nym/pull/1081) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump url-parse from 1.5.1 to 1.5.4 in /clients/native/examples/js-examples/websocket [\#1080](https://github.com/nymtech/nym/pull/1080) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump follow-redirects from 1.14.1 to 1.14.7 in /clients/native/examples/js-examples/websocket [\#1079](https://github.com/nymtech/nym/pull/1079) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump nanoid from 3.1.23 to 3.2.0 in /clients/native/examples/js-examples/websocket [\#1078](https://github.com/nymtech/nym/pull/1078) ([dependabot[bot]](https://github.com/apps/dependabot))
- Setup basic test for mixnode stats reporting [\#1077](https://github.com/nymtech/nym/pull/1077) ([octol](https://github.com/octol))
- Make wallet_address mandatory for mixnode init [\#1076](https://github.com/nymtech/nym/pull/1076) ([octol](https://github.com/octol))
- Tidy nym-mixnode module visibility [\#1075](https://github.com/nymtech/nym/pull/1075) ([octol](https://github.com/octol))
- Feature/wallet login with password [\#1074](https://github.com/nymtech/nym/pull/1074) ([fmtabbara](https://github.com/fmtabbara))
- Add trait to mock client dependency in DelayForwarder [\#1073](https://github.com/nymtech/nym/pull/1073) ([octol](https://github.com/octol))
- Bump rust-version to latest stable for nym-mixnode [\#1072](https://github.com/nymtech/nym/pull/1072) ([octol](https://github.com/octol))
- Fixes CI for our wasm build [\#1069](https://github.com/nymtech/nym/pull/1069) ([jstuczyn](https://github.com/jstuczyn))
- Add @octol as codeowner [\#1068](https://github.com/nymtech/nym/pull/1068) ([octol](https://github.com/octol))
- set-up inclusion probability [\#1067](https://github.com/nymtech/nym/pull/1067) ([fmtabbara](https://github.com/fmtabbara))
- Feature/wasm client [\#1066](https://github.com/nymtech/nym/pull/1066) ([neacsu](https://github.com/neacsu))
- Changed bech32_prefix from punk to nymt [\#1064](https://github.com/nymtech/nym/pull/1064) ([jstuczyn](https://github.com/jstuczyn))
- Bump nanoid from 3.1.30 to 3.2.0 in /testnet-faucet [\#1063](https://github.com/nymtech/nym/pull/1063) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump nanoid from 3.1.30 to 3.2.0 in /nym-wallet [\#1062](https://github.com/nymtech/nym/pull/1062) ([dependabot[bot]](https://github.com/apps/dependabot))
- Rework vesting contract storage [\#1061](https://github.com/nymtech/nym/pull/1061) ([durch](https://github.com/durch))
- Mixnet Contract constants extraction [\#1060](https://github.com/nymtech/nym/pull/1060) ([jstuczyn](https://github.com/jstuczyn))
- fix: make explorer footer year dynamic [\#1059](https://github.com/nymtech/nym/pull/1059) ([martinyung](https://github.com/martinyung))
- Add mnemonic just on creation, to display it [\#1057](https://github.com/nymtech/nym/pull/1057) ([neacsu](https://github.com/neacsu))
- Network Explorer: updates to API and UI to show the active set [\#1056](https://github.com/nymtech/nym/pull/1056) ([mmsinclair](https://github.com/mmsinclair))
- Made contract addresses for query NyxdClient construction optional [\#1055](https://github.com/nymtech/nym/pull/1055) ([jstuczyn](https://github.com/jstuczyn))
- Introduced RPC query for total token supply [\#1053](https://github.com/nymtech/nym/pull/1053) ([jstuczyn](https://github.com/jstuczyn))
- Feature/tokio console [\#1052](https://github.com/nymtech/nym/pull/1052) ([durch](https://github.com/durch))
- Implemented beta clippy lint recommendations [\#1051](https://github.com/nymtech/nym/pull/1051) ([jstuczyn](https://github.com/jstuczyn))
- add new function to update profit percentage [\#1050](https://github.com/nymtech/nym/pull/1050) ([fmtabbara](https://github.com/fmtabbara))
- Upgrade Clap and use declarative argument parsing for nym-mixnode [\#1047](https://github.com/nymtech/nym/pull/1047) ([octol](https://github.com/octol))
- Feature/additional bond validation [\#1046](https://github.com/nymtech/nym/pull/1046) ([fmtabbara](https://github.com/fmtabbara))
- Fix clippy on relevant lints [\#1044](https://github.com/nymtech/nym/pull/1044) ([neacsu](https://github.com/neacsu))
- Bump shelljs from 0.8.4 to 0.8.5 in /contracts/basic-bandwidth-generation [\#1043](https://github.com/nymtech/nym/pull/1043) ([dependabot[bot]](https://github.com/apps/dependabot))
- Endpoint for rewarded set inclusion probabilities [\#1042](https://github.com/nymtech/nym/pull/1042) ([durch](https://github.com/durch))
- Bump follow-redirects from 1.14.4 to 1.14.7 in /contracts/basic-bandwidth-generation [\#1041](https://github.com/nymtech/nym/pull/1041) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump follow-redirects from 1.14.5 to 1.14.7 in /testnet-faucet [\#1040](https://github.com/nymtech/nym/pull/1040) ([dependabot[bot]](https://github.com/apps/dependabot))
- Feature/node settings update [\#1036](https://github.com/nymtech/nym/pull/1036) ([fmtabbara](https://github.com/fmtabbara))
- Migrate to cw-storage-plus 0.11.1 [\#1035](https://github.com/nymtech/nym/pull/1035) ([durch](https://github.com/durch))
- Bump @openzeppelin/contracts from 4.4.1 to 4.4.2 in /contracts/basic-bandwidth-generation [\#1034](https://github.com/nymtech/nym/pull/1034) ([dependabot[bot]](https://github.com/apps/dependabot))
- Feature/configurable wallet [\#1033](https://github.com/nymtech/nym/pull/1033) ([neacsu](https://github.com/neacsu))
- Feature/downcast reward estimation [\#1031](https://github.com/nymtech/nym/pull/1031) ([durch](https://github.com/durch))
- Wallet UI updates [\#1028](https://github.com/nymtech/nym/pull/1028) ([fmtabbara](https://github.com/fmtabbara))
- Remove migration code [\#1027](https://github.com/nymtech/nym/pull/1027) ([neacsu](https://github.com/neacsu))
- Chore/stricter dependency requirements [\#1025](https://github.com/nymtech/nym/pull/1025) ([jstuczyn](https://github.com/jstuczyn))
- Feature/validator api client endpoints [\#1024](https://github.com/nymtech/nym/pull/1024) ([jstuczyn](https://github.com/jstuczyn))
- Updated cosmrs to 0.4.1 [\#1023](https://github.com/nymtech/nym/pull/1023) ([jstuczyn](https://github.com/jstuczyn))
- Feature/testnet deploy scripts [\#1022](https://github.com/nymtech/nym/pull/1022) ([mfahampshire](https://github.com/mfahampshire))
- Changed wallet's client to a full validator client [\#1021](https://github.com/nymtech/nym/pull/1021) ([jstuczyn](https://github.com/jstuczyn))
- Fix 404 link [\#1020](https://github.com/nymtech/nym/pull/1020) ([RiccardoMasutti](https://github.com/RiccardoMasutti))
- Feature/additional mixnode endpoints [\#1019](https://github.com/nymtech/nym/pull/1019) ([jstuczyn](https://github.com/jstuczyn))
- Introduced denom check when trying to withdraw vested coins [\#1018](https://github.com/nymtech/nym/pull/1018) ([jstuczyn](https://github.com/jstuczyn))
- Add network defaults for qa [\#1017](https://github.com/nymtech/nym/pull/1017) ([neacsu](https://github.com/neacsu))
- Feature/expanded events [\#1015](https://github.com/nymtech/nym/pull/1015) ([jstuczyn](https://github.com/jstuczyn))
- update frontend to use new profit update api [\#1014](https://github.com/nymtech/nym/pull/1014) ([fmtabbara](https://github.com/fmtabbara))
- Feature/node state endpoint [\#1013](https://github.com/nymtech/nym/pull/1013) ([jstuczyn](https://github.com/jstuczyn))
- Feature/hourly set updates [\#1012](https://github.com/nymtech/nym/pull/1012) ([durch](https://github.com/durch))
- Feature/remove unused profit margin [\#1011](https://github.com/nymtech/nym/pull/1011) ([neacsu](https://github.com/neacsu))
- Feature/explorer node status [\#1010](https://github.com/nymtech/nym/pull/1010) ([jstuczyn](https://github.com/jstuczyn))
- Use serial integer instead of random [\#1009](https://github.com/nymtech/nym/pull/1009) ([durch](https://github.com/durch))
- Feature/configure profit [\#1008](https://github.com/nymtech/nym/pull/1008) ([neacsu](https://github.com/neacsu))
- Feature/fix gateway sign [\#1004](https://github.com/nymtech/nym/pull/1004) ([neacsu](https://github.com/neacsu))
- Fix clippy [\#1003](https://github.com/nymtech/nym/pull/1003) ([neacsu](https://github.com/neacsu))
- Update wallet version [\#998](https://github.com/nymtech/nym/pull/998) ([tommyv1987](https://github.com/tommyv1987))
- Fix wallet build instructions [\#997](https://github.com/nymtech/nym/pull/997) ([tommyv1987](https://github.com/tommyv1987))
- Make the separation between testnet-mode and erc20 bandwidth mode clearer [\#994](https://github.com/nymtech/nym/pull/994) ([neacsu](https://github.com/neacsu))
- Bump @openzeppelin/contracts from 3.4.0 to 4.4.1 in /contracts/basic-bandwidth-generation [\#983](https://github.com/nymtech/nym/pull/983) ([dependabot[bot]](https://github.com/apps/dependabot))
- Feature/implicit runtime [\#973](https://github.com/nymtech/nym/pull/973) ([jstuczyn](https://github.com/jstuczyn))
- Differentiate staking and ownership [\#961](https://github.com/nymtech/nym/pull/961) ([durch](https://github.com/durch))
