# Changelog

Post 1.0.0 release, the changelog format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- nym-connect: initial proof-of-concept of a UI around the socks5 client was added.
- all: added network compilation target to `--help` (or `--version`) commands ([#1256]).
- explorer-api: learned how to sum the delegations by owner in a new endpoint.
- gateway: Added gateway coconut verifications and validator-api communication for double spending protection ([#1261])
- mixnet-contract: Added ClaimOperatorReward and ClaimDelegatorReward messages ([#1292])
- mixnet-contract: Replace all naked `-` with `saturating_sub`.
- mixnet-contrat: Added staking_supply field to ContractStateParams.
- network-explorer-ui: Upgrade to React Router 6
- rewarding: replace circulating supply with staking supply in reward calculations ([#1324])
- validator-api: add `estimated_node_profit` and `estimated_operator_cost` to `reward-estimate` endpoint ([#1284])
- validator-api: add detailed mixnode bond endpoints, and explorer-api makes use of that data to append stake saturation.
- validator-api: add Swagger to document the REST API ([#1249]).
- validator-api: Added new endpoints for coconut spending flow and communications with coconut & multisig contracts ([#1261])
- vesting-contract: Added ClaimOperatorReward and ClaimDelegatorReward messages ([#1292])
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
- network-statistics: a new mixnet service that aggregates and exposes anonymized data about mixnet services ([#1328])

### Fixed

- mixnet-contract: `estimated_delegator_reward` calculation ([#1284])
- mixnet-contract: delegator and operator rewards use lambda and sigma instead of lambda_ticked and sigma_ticked ([#1284])
- mixnet-contract: removed `expect` in `query_delegator_reward` and queries containing invalid proxy address should now return a more human-readable error ([#1257])
- mixnet-contract: replaced integer division with fixed for performance calculations ([#1284])
- mixnet-contract: Under certain circumstances nodes could not be unbonded ([#1255](https://github.com/nymtech/nym/issues/1255)) ([#1258])
- mixnode, gateway: attempting to determine reconnection backoff to persistently failing mixnode could result in a crash ([#1260])
- mixnode: the mixnode learned how to shutdown gracefully.
- vesting-contract: replaced `checked_sub` with `saturating_sub` to fix the underflow in `get_vesting_tokens` ([#1275])
- native & socks5 clients: fail early when clients try to re-init with a different gateway, which is not supported yet ([#1322])
- validator: fixed local docker-compose setup to work on Apple M1 ([#1329])
- wallet: overwritten the `32x32.png` icon with a higher resolution variant to temporarily fix icon issues on Linux (#[1354]) 

### Changed

- validator-client: created internal `Coin` type that replaces coins from `cosmrs` and `cosmwasm` for API entrypoints [[#1295]]
- all: updated all `cosmwasm`-related dependencies to `1.0.0` and `cw-storage-plus` to `0.13.4` [[#1318]]
- network-requester: allow to voluntarily store and send statistical data about the number of bytes the proxied server serves ([#1328])
- wallet: updated backend to `tauri v1.0.0-rc.14` (#[1354])

[#1249]: https://github.com/nymtech/nym/pull/1249
[#1256]: https://github.com/nymtech/nym/pull/1256
[#1257]: https://github.com/nymtech/nym/pull/1257
[#1258]: https://github.com/nymtech/nym/pull/1258
[#1260]: https://github.com/nymtech/nym/pull/1260
[#1261]: https://github.com/nymtech/nym/pull/1261
[#1265]: https://github.com/nymtech/nym/pull/1265
[#1267]: https://github.com/nymtech/nym/pull/1267
[#1275]: https://github.com/nymtech/nym/pull/1275
[#1278]: https://github.com/nymtech/nym/pull/1278
[#1284]: https://github.com/nymtech/nym/pull/1284
[#1292]: https://github.com/nymtech/nym/pull/1292
[#1295]: https://github.com/nymtech/nym/pull/1295
[#1302]: https://github.com/nymtech/nym/pull/1302
[#1318]: https://github.com/nymtech/nym/pull/1318
[#1322]: https://github.com/nymtech/nym/pull/1322
[#1324]: https://github.com/nymtech/nym/pull/1324
[#1328]: https://github.com/nymtech/nym/pull/1328
[#1329]: https://github.com/nymtech/nym/pull/1329
[#1354]: https://github.com/nymtech/nym/pull/1354

## [nym-wallet-v1.0.4](https://github.com/nymtech/nym/tree/nym-wallet-v1.0.4) (2022-05-04)

### Changed

- all: the default behaviour of validator client is changed to use `broadcast_sync` and poll for transaction inclusion instead of using `broadcast_commit` to deal with timeouts ([#1246])

## [v1.0.1](https://github.com/nymtech/nym/tree/v1.0.1) (2022-05-04)

### Added

- validator-api: introduced endpoint for getting average mixnode uptime ([#1238])

### Changed

- all: the default behaviour of validator client is changed to use `broadcast_sync` and poll for transaction inclusion instead of using `broadcast_commit` to deal with timeouts ([#1246])

### Fixed

- nym-network-requester: is included in the Github Actions for building release binaries

[#1238]: https://github.com/nymtech/nym/pull/1238
[#1246]: https://github.com/nymtech/nym/pull/1246

## [v1.0.0](https://github.com/nymtech/nym/tree/v1.0.0) (2022-05-03)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.12.1...v1.0.0)

**Merged pull requests:**

- Feature/show pending delegations [\#1229](https://github.com/nymtech/nym/pull/1229) ([fmtabbara](https://github.com/fmtabbara))
- Bucket inclusion probabilities [\#1224](https://github.com/nymtech/nym/pull/1224) ([durch](https://github.com/durch))
- Create a new bundled delegation when compounding rewards [\#1221](https://github.com/nymtech/nym/pull/1221) ([durch](https://github.com/durch))

## [nym-binaries-1.0.0](https://github.com/nymtech/nym/tree/nym-binaries-1.0.0) (2022-04-27)

[Full Changelog](https://github.com/nymtech/nym/compare/nym-wallet-v1.0.3...nym-binaries-1.0.0)

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
- Bump ansi-regex from 4.1.0 to 4.1.1 in /docker/typescript\_client/upload\_contract [\#1171](https://github.com/nymtech/nym/pull/1171) ([dependabot[bot]](https://github.com/apps/dependabot))

## [nym-binaries-1.0.0-rc.2](https://github.com/nymtech/nym/tree/nym-binaries-1.0.0-rc.2) (2022-04-15)

[Full Changelog](https://github.com/nymtech/nym/compare/nym-wallet-v1.0.2...nym-binaries-1.0.0-rc.2)

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
- wallet: add validate\_mnemonic [\#1186](https://github.com/nymtech/nym/pull/1186) ([octol](https://github.com/octol))
- wallet: support removing accounts from the wallet file [\#1185](https://github.com/nymtech/nym/pull/1185) ([octol](https://github.com/octol))
- Feature/adding discord [\#1184](https://github.com/nymtech/nym/pull/1184) ([gala1234](https://github.com/gala1234))
- wallet: config backend for validator selection [\#1183](https://github.com/nymtech/nym/pull/1183) ([octol](https://github.com/octol))
- Add storybook to wallet [\#1178](https://github.com/nymtech/nym/pull/1178) ([mmsinclair](https://github.com/mmsinclair))
- wallet: connection test nymd and api urls independently [\#1170](https://github.com/nymtech/nym/pull/1170) ([octol](https://github.com/octol))
- wallet: wire up account storage [\#1153](https://github.com/nymtech/nym/pull/1153) ([octol](https://github.com/octol))
- Feature/signature on deposit [\#1151](https://github.com/nymtech/nym/pull/1151) ([neacsu](https://github.com/neacsu))

## [nym-wallet-v1.0.1](https://github.com/nymtech/nym/tree/nym-wallet-v1.0.1) (2022-04-05)

[Full Changelog](https://github.com/nymtech/nym/compare/nym-binaries-1.0.0-rc.1...nym-wallet-v1.0.1)

**Closed issues:**

- Check enabling bbbc simultaneously with open access. Estimate what it would take to make this the default compilation target. [\#1175](https://github.com/nymtech/nym/issues/1175)
- Get coconut credential for deposited tokens [\#1138](https://github.com/nymtech/nym/issues/1138)
- Make payments lazy [\#1135](https://github.com/nymtech/nym/issues/1135)
- Uptime on node selection for sets [\#1049](https://github.com/nymtech/nym/issues/1049)

## [nym-binaries-1.0.0-rc.1](https://github.com/nymtech/nym/tree/nym-binaries-1.0.0-rc.1) (2022-03-28)

[Full Changelog](https://github.com/nymtech/nym/compare/nym-wallet-v1.0.0...nym-binaries-1.0.0-rc.1)

**Fixed bugs:**

- \[Issue\]cargo build --release issue [\#1101](https://github.com/nymtech/nym/issues/1101)
- appimage fail to load in Fedora [\#1098](https://github.com/nymtech/nym/issues/1098)
- \[Issue\] React Example project does not compile when using @nymproject/nym-client-wasm v0.9.0-1 [\#878](https://github.com/nymtech/nym/issues/878)

**Closed issues:**

- Make mainnet coin transfers work [\#1096](https://github.com/nymtech/nym/issues/1096)
- Make Nym wallet validators configurable at runtime [\#1026](https://github.com/nymtech/nym/issues/1026)
- Project Platypus e2e / integration testing [\#942](https://github.com/nymtech/nym/issues/942)
- \[Coconut\]: Replace ElGamal with Pedersen commitments [\#901](https://github.com/nymtech/nym/issues/901)

**Merged pull requests:**

- Different values for mixes and gateways [\#1169](https://github.com/nymtech/nym/pull/1169) ([durch](https://github.com/durch))
- Add global blacklist to validator-cache [\#1168](https://github.com/nymtech/nym/pull/1168) ([durch](https://github.com/durch))
- Feature/upgrade rewarding sandbox [\#1167](https://github.com/nymtech/nym/pull/1167) ([durch](https://github.com/durch))
- Bump node-forge from 1.2.1 to 1.3.0 [\#1165](https://github.com/nymtech/nym/pull/1165) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump minimist from 1.2.5 to 1.2.6 in /nym-wallet/webdriver [\#1164](https://github.com/nymtech/nym/pull/1164) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump minimist from 1.2.5 to 1.2.6 in /clients/tauri-client [\#1163](https://github.com/nymtech/nym/pull/1163) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump minimist from 1.2.5 to 1.2.6 in /clients/webassembly/js-example [\#1162](https://github.com/nymtech/nym/pull/1162) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump minimist from 1.2.5 to 1.2.6 in /clients/native/examples/js-examples/websocket [\#1160](https://github.com/nymtech/nym/pull/1160) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump minimist from 1.2.5 to 1.2.6 in /docker/typescript\_client/upload\_contract [\#1159](https://github.com/nymtech/nym/pull/1159) ([dependabot[bot]](https://github.com/apps/dependabot))
- Feature/vesting full [\#1158](https://github.com/nymtech/nym/pull/1158) ([fmtabbara](https://github.com/fmtabbara))
- get\_current\_epoch tauri [\#1156](https://github.com/nymtech/nym/pull/1156) ([durch](https://github.com/durch))
- Cleanup [\#1155](https://github.com/nymtech/nym/pull/1155) ([durch](https://github.com/durch))
- Feature flag reward payments [\#1154](https://github.com/nymtech/nym/pull/1154) ([durch](https://github.com/durch))
- Add Query endpoints for calculating rewards [\#1152](https://github.com/nymtech/nym/pull/1152) ([durch](https://github.com/durch))
- Pending endpoints [\#1150](https://github.com/nymtech/nym/pull/1150) ([durch](https://github.com/durch))
- wallet: add logging [\#1149](https://github.com/nymtech/nym/pull/1149) ([octol](https://github.com/octol))
- wallet: use Urls rather than Strings for validator urls [\#1148](https://github.com/nymtech/nym/pull/1148) ([octol](https://github.com/octol))
- Change accumulated reward to Option, migrate delegations [\#1147](https://github.com/nymtech/nym/pull/1147) ([durch](https://github.com/durch))
- wallet: fetch validators url remotely if available [\#1146](https://github.com/nymtech/nym/pull/1146) ([octol](https://github.com/octol))
- Fix delegated\_free calculation [\#1145](https://github.com/nymtech/nym/pull/1145) ([durch](https://github.com/durch))
- Update Nym wallet dependencies to use `ts-packages` [\#1144](https://github.com/nymtech/nym/pull/1144) ([mmsinclair](https://github.com/mmsinclair))
- wallet: try validators one by one if available [\#1143](https://github.com/nymtech/nym/pull/1143) ([octol](https://github.com/octol))
- Update Network Explorer Packages and add mix node identity key copy [\#1142](https://github.com/nymtech/nym/pull/1142) ([mmsinclair](https://github.com/mmsinclair))
- Feature/vesting token pool selector [\#1140](https://github.com/nymtech/nym/pull/1140) ([fmtabbara](https://github.com/fmtabbara))
- Add `ts-packages` for shared Typescript packages [\#1139](https://github.com/nymtech/nym/pull/1139) ([mmsinclair](https://github.com/mmsinclair))
- allow main-net prefix and denom to work [\#1137](https://github.com/nymtech/nym/pull/1137) ([tommyv1987](https://github.com/tommyv1987))
- Upgrade blake3 to v1.3.1 and tauri to 1.0.0-rc.3 [\#1136](https://github.com/nymtech/nym/pull/1136) ([mmsinclair](https://github.com/mmsinclair))
- Bump url-parse from 1.5.7 to 1.5.10 in /clients/native/examples/js-examples/websocket [\#1134](https://github.com/nymtech/nym/pull/1134) ([dependabot[bot]](https://github.com/apps/dependabot))
- Use network explorer map data with disputed areas [\#1133](https://github.com/nymtech/nym/pull/1133) ([Baro1905](https://github.com/Baro1905))
- Feature/vesting UI [\#1132](https://github.com/nymtech/nym/pull/1132) ([fmtabbara](https://github.com/fmtabbara))
- Refactor to a lazy rewarding system [\#1127](https://github.com/nymtech/nym/pull/1127) ([durch](https://github.com/durch))
- Bump ws from 6.2.1 to 6.2.2 in /clients/webassembly/js-example [\#1126](https://github.com/nymtech/nym/pull/1126) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump url-parse from 1.4.7 to 1.5.7 in /clients/webassembly/react-example [\#1125](https://github.com/nymtech/nym/pull/1125) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump url-parse from 1.5.4 to 1.5.7 in /clients/native/examples/js-examples/websocket [\#1124](https://github.com/nymtech/nym/pull/1124) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump url-parse from 1.5.1 to 1.5.7 in /clients/webassembly/js-example [\#1122](https://github.com/nymtech/nym/pull/1122) ([dependabot[bot]](https://github.com/apps/dependabot))
- update contract address [\#1121](https://github.com/nymtech/nym/pull/1121) ([tommyv1987](https://github.com/tommyv1987))
- Refactor GitHub Actions notifications [\#1119](https://github.com/nymtech/nym/pull/1119) ([mmsinclair](https://github.com/mmsinclair))
- Change `pledge` to `bond` in gateway list [\#1118](https://github.com/nymtech/nym/pull/1118) ([mmsinclair](https://github.com/mmsinclair))
- Bump follow-redirects from 1.14.7 to 1.14.8 in /contracts/basic-bandwidth-generation [\#1117](https://github.com/nymtech/nym/pull/1117) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump follow-redirects from 1.14.3 to 1.14.8 in /explorer [\#1116](https://github.com/nymtech/nym/pull/1116) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump follow-redirects from 1.14.5 to 1.14.8 in /nym-wallet [\#1115](https://github.com/nymtech/nym/pull/1115) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump follow-redirects from 1.14.7 to 1.14.8 in /clients/native/examples/js-examples/websocket [\#1114](https://github.com/nymtech/nym/pull/1114) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump follow-redirects from 1.14.7 to 1.14.8 in /testnet-faucet [\#1113](https://github.com/nymtech/nym/pull/1113) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump follow-redirects from 1.14.1 to 1.14.8 in /clients/webassembly/js-example [\#1112](https://github.com/nymtech/nym/pull/1112) ([dependabot[bot]](https://github.com/apps/dependabot))
- Feature/vesting get current period [\#1111](https://github.com/nymtech/nym/pull/1111) ([durch](https://github.com/durch))
- Bump simple-get from 2.8.1 to 2.8.2 in /contracts/basic-bandwidth-generation [\#1110](https://github.com/nymtech/nym/pull/1110) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump simple-get from 3.1.0 to 3.1.1 in /explorer [\#1109](https://github.com/nymtech/nym/pull/1109) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump simple-get from 3.1.0 to 3.1.1 in /clients/tauri-client [\#1108](https://github.com/nymtech/nym/pull/1108) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump simple-get from 3.1.0 to 3.1.1 in /nym-wallet [\#1107](https://github.com/nymtech/nym/pull/1107) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump node-sass from 4.14.1 to 7.0.0 in /clients/webassembly/react-example [\#1105](https://github.com/nymtech/nym/pull/1105) ([dependabot[bot]](https://github.com/apps/dependabot))
- Fix hardcoded period logic [\#1104](https://github.com/nymtech/nym/pull/1104) ([durch](https://github.com/durch))
- Fixed underflow in rewarding all delegators [\#1099](https://github.com/nymtech/nym/pull/1099) ([jstuczyn](https://github.com/jstuczyn))
- Emit original bond as part of rewarding event [\#1094](https://github.com/nymtech/nym/pull/1094) ([jstuczyn](https://github.com/jstuczyn))
- Add UpdateMixnodeConfigOnBehalf to vestng contract [\#1091](https://github.com/nymtech/nym/pull/1091) ([durch](https://github.com/durch))
- Fixes infinite loops in requests involving pagination [\#1085](https://github.com/nymtech/nym/pull/1085) ([jstuczyn](https://github.com/jstuczyn))
- Removes migration code [\#1071](https://github.com/nymtech/nym/pull/1071) ([jstuczyn](https://github.com/jstuczyn))
- feature/pedersen-commitments [\#1048](https://github.com/nymtech/nym/pull/1048) ([danielementary](https://github.com/danielementary))
- Feature/reuse init owner [\#970](https://github.com/nymtech/nym/pull/970) ([neacsu](https://github.com/neacsu))

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
- Update cw-storage-plus to 0.11  [\#1032](https://github.com/nymtech/nym/issues/1032)
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
- Make wallet\_address mandatory for mixnode init [\#1076](https://github.com/nymtech/nym/pull/1076) ([octol](https://github.com/octol))
- Tidy nym-mixnode module visibility [\#1075](https://github.com/nymtech/nym/pull/1075) ([octol](https://github.com/octol))
- Feature/wallet login with password [\#1074](https://github.com/nymtech/nym/pull/1074) ([fmtabbara](https://github.com/fmtabbara))
- Add trait to mock client dependency in DelayForwarder [\#1073](https://github.com/nymtech/nym/pull/1073) ([octol](https://github.com/octol))
- Bump rust-version to latest stable for nym-mixnode [\#1072](https://github.com/nymtech/nym/pull/1072) ([octol](https://github.com/octol))
- Fixes CI for our wasm build [\#1069](https://github.com/nymtech/nym/pull/1069) ([jstuczyn](https://github.com/jstuczyn))
- Add @octol as codeowner [\#1068](https://github.com/nymtech/nym/pull/1068) ([octol](https://github.com/octol))
- set-up inclusion probability [\#1067](https://github.com/nymtech/nym/pull/1067) ([fmtabbara](https://github.com/fmtabbara))
- Feature/wasm client [\#1066](https://github.com/nymtech/nym/pull/1066) ([neacsu](https://github.com/neacsu))
- Changed bech32\_prefix from punk to nymt [\#1064](https://github.com/nymtech/nym/pull/1064) ([jstuczyn](https://github.com/jstuczyn))
- Bump nanoid from 3.1.30 to 3.2.0 in /testnet-faucet [\#1063](https://github.com/nymtech/nym/pull/1063) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump nanoid from 3.1.30 to 3.2.0 in /nym-wallet [\#1062](https://github.com/nymtech/nym/pull/1062) ([dependabot[bot]](https://github.com/apps/dependabot))
- Rework vesting contract storage [\#1061](https://github.com/nymtech/nym/pull/1061) ([durch](https://github.com/durch))
- Mixnet Contract constants extraction [\#1060](https://github.com/nymtech/nym/pull/1060) ([jstuczyn](https://github.com/jstuczyn))
- fix: make explorer footer year dynamic [\#1059](https://github.com/nymtech/nym/pull/1059) ([martinyung](https://github.com/martinyung))
- Add mnemonic just on creation, to display it [\#1057](https://github.com/nymtech/nym/pull/1057) ([neacsu](https://github.com/neacsu))
- Network Explorer: updates to API and UI to show the active set [\#1056](https://github.com/nymtech/nym/pull/1056) ([mmsinclair](https://github.com/mmsinclair))
- Made contract addresses for query NymdClient construction optional [\#1055](https://github.com/nymtech/nym/pull/1055) ([jstuczyn](https://github.com/jstuczyn))
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

## [v0.12.1](https://github.com/nymtech/nym/tree/v0.12.1) (2021-12-23)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.12.0...v0.12.1)

**Implemented enhancements:**

- Add version check to  binaries [\#967](https://github.com/nymtech/nym/issues/967)

**Fixed bugs:**

- \[Issue\] NYM wallet doesn't work after login [\#995](https://github.com/nymtech/nym/issues/995)
- \[Issue\] [\#993](https://github.com/nymtech/nym/issues/993)
- NYM wallet setup trouble\[Issue\] [\#958](https://github.com/nymtech/nym/issues/958)

## [v0.12.0](https://github.com/nymtech/nym/tree/v0.12.0) (2021-12-21)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.11.0...v0.12.0)

**Implemented enhancements:**

- Introduces query for contract build information [\#919](https://github.com/nymtech/nym/pull/919) ([jstuczyn](https://github.com/jstuczyn))

**Fixed bugs:**

- Mixnodes - claim tokens scenario does not work with telegram bot [\#938](https://github.com/nymtech/nym/issues/938)
- \[Issue\]"create account" button does not work on Ubuntu 20.04.03 LTS [\#916](https://github.com/nymtech/nym/issues/916)
- \[Issue\] NodeJS 17.1.0 and webpack issues causing nym-wallet build to fail on Pop!OS 21.04\(Ubuntu\) [\#914](https://github.com/nymtech/nym/issues/914)
- Prevent overwriting of Mixnodes if the mixnode is already bonded [\#912](https://github.com/nymtech/nym/issues/912)
- Pasting mnemonic doesn't work on MacOS [\#908](https://github.com/nymtech/nym/issues/908)
- Wallet - investigate nav freezes [\#716](https://github.com/nymtech/nym/issues/716)
- Wallet - Fix console errors [\#707](https://github.com/nymtech/nym/issues/707)
- Fixed invalid nodes being counted twice in unroutable category [\#963](https://github.com/nymtech/nym/pull/963) ([jstuczyn](https://github.com/jstuczyn))
- Don't reset total delegation on mixnode rebond [\#940](https://github.com/nymtech/nym/pull/940) ([jstuczyn](https://github.com/jstuczyn))
- Bugfix/remove mixnode bonding overwrite [\#917](https://github.com/nymtech/nym/pull/917) ([jstuczyn](https://github.com/jstuczyn))
- Fixes crash condition in validator API when calculating last day uptime [\#909](https://github.com/nymtech/nym/pull/909) ([jstuczyn](https://github.com/jstuczyn))
- Bugfix/monitor initial values wait [\#907](https://github.com/nymtech/nym/pull/907) ([jstuczyn](https://github.com/jstuczyn))
- Bug fix: Network Explorer: Add freegeoip API key and split out tasks for country distributions  [\#806](https://github.com/nymtech/nym/pull/806) ([mmsinclair](https://github.com/mmsinclair))
- Explorer API: port test now split out address resolution and add units tests [\#755](https://github.com/nymtech/nym/pull/755) ([mmsinclair](https://github.com/mmsinclair))

**Closed issues:**

- Feature gate `ts-rs` everywhere and only build use it to export types during CI runs [\#893](https://github.com/nymtech/nym/issues/893)
- Error when init Nym client for Nym requester [\#800](https://github.com/nymtech/nym/issues/800)
- Website updates - Add new team members and translations [\#775](https://github.com/nymtech/nym/issues/775)
- Update Run Nym Nodes Documentation [\#773](https://github.com/nymtech/nym/issues/773)
- Upgrade `prost` to 0.8 [\#768](https://github.com/nymtech/nym/issues/768)
- How can I get 100punk\(Version: 0.11.0\) [\#743](https://github.com/nymtech/nym/issues/743)
- Wallet - Fix Bond Form validation issue [\#717](https://github.com/nymtech/nym/issues/717)
- help!!! [\#712](https://github.com/nymtech/nym/issues/712)
- UX feature request: show all delegated nodes in wallet [\#711](https://github.com/nymtech/nym/issues/711)
- UX feature request: add current balance on wallet pages [\#710](https://github.com/nymtech/nym/issues/710)
- got sign issue from bot  [\#709](https://github.com/nymtech/nym/issues/709)
- As a wallet user, I would like to be able to log out of the wallet [\#706](https://github.com/nymtech/nym/issues/706)
- As a wallet user, I would like to have a "receive" page where I can see my own wallet address [\#705](https://github.com/nymtech/nym/issues/705)
- Update native client/socks client/mixnode/gateway `upgrade` command [\#689](https://github.com/nymtech/nym/issues/689)
- Update mixnode/gateway/client to use query for cached nodes rather than use validator [\#688](https://github.com/nymtech/nym/issues/688)
- '--directory' not expected error starting local mixnet [\#520](https://github.com/nymtech/nym/issues/520)
- nym-socks5-client is painfully slow [\#495](https://github.com/nymtech/nym/issues/495)
- nym-socks5-client crash after opening Keybase team "Browse all channels" [\#494](https://github.com/nymtech/nym/issues/494)
- Mixed Content problem [\#400](https://github.com/nymtech/nym/issues/400)
- Gateway disk quota [\#137](https://github.com/nymtech/nym/issues/137)
- Simplify message encapsulation with regards to topology  [\#127](https://github.com/nymtech/nym/issues/127)
- Create constants for cli argument names [\#115](https://github.com/nymtech/nym/issues/115)
- Using Blake3 as a hash function [\#103](https://github.com/nymtech/nym/issues/103)
- Validator should decide which layer a node is in [\#86](https://github.com/nymtech/nym/issues/86)
- Clean shutdown for all processes [\#73](https://github.com/nymtech/nym/issues/73)
- Client API consistency [\#71](https://github.com/nymtech/nym/issues/71)
- Simplify concurrency with a proper actor framework [\#31](https://github.com/nymtech/nym/issues/31)
- Database for gateway [\#11](https://github.com/nymtech/nym/issues/11)

**Merged pull requests:**

- Update wallet to align with versioning on nodes and gateways [\#991](https://github.com/nymtech/nym/pull/991) ([tommyv1987](https://github.com/tommyv1987))
- Fix success view messages. [\#990](https://github.com/nymtech/nym/pull/990) ([tommyv1987](https://github.com/tommyv1987))
- Feature/enable signature check [\#989](https://github.com/nymtech/nym/pull/989) ([neacsu](https://github.com/neacsu))
- Update mixnet contract address [\#988](https://github.com/nymtech/nym/pull/988) ([neacsu](https://github.com/neacsu))
- Fix verloc print [\#987](https://github.com/nymtech/nym/pull/987) ([neacsu](https://github.com/neacsu))
- Feature/refactor mixnet contract test helpers [\#986](https://github.com/nymtech/nym/pull/986) ([futurechimp](https://github.com/futurechimp))
- Making the terminology consistent between mixnode/gateway output and … [\#985](https://github.com/nymtech/nym/pull/985) ([futurechimp](https://github.com/futurechimp))
- Feature/add wallet to gateway init [\#984](https://github.com/nymtech/nym/pull/984) ([futurechimp](https://github.com/futurechimp))
- Feature/add wallet address to init [\#982](https://github.com/nymtech/nym/pull/982) ([futurechimp](https://github.com/futurechimp))
- Update message to bond mixnode [\#981](https://github.com/nymtech/nym/pull/981) ([tommyv1987](https://github.com/tommyv1987))
- Bump version to 0.12.0 [\#980](https://github.com/nymtech/nym/pull/980) ([neacsu](https://github.com/neacsu))
- Feature/rename erc20 [\#979](https://github.com/nymtech/nym/pull/979) ([neacsu](https://github.com/neacsu))
- Removed web wallet [\#978](https://github.com/nymtech/nym/pull/978) ([futurechimp](https://github.com/futurechimp))
- Network Explorer: fix uptime history display to use new API response [\#977](https://github.com/nymtech/nym/pull/977) ([mmsinclair](https://github.com/mmsinclair))
- Make develop branch agnostic of the network [\#976](https://github.com/nymtech/nym/pull/976) ([neacsu](https://github.com/neacsu))
- Fix windows fmt [\#975](https://github.com/nymtech/nym/pull/975) ([neacsu](https://github.com/neacsu))
- Feature/wallet settings area [\#974](https://github.com/nymtech/nym/pull/974) ([fmtabbara](https://github.com/fmtabbara))
- Feature/node info command [\#972](https://github.com/nymtech/nym/pull/972) ([jstuczyn](https://github.com/jstuczyn))
- Use the renamed balance function [\#971](https://github.com/nymtech/nym/pull/971) ([neacsu](https://github.com/neacsu))
- Introduced 'version' command to all relevant binaries [\#969](https://github.com/nymtech/nym/pull/969) ([jstuczyn](https://github.com/jstuczyn))
- Feature/new testnet wallet updates [\#968](https://github.com/nymtech/nym/pull/968) ([fmtabbara](https://github.com/fmtabbara))
- Feature/optional bandwidth bypass [\#965](https://github.com/nymtech/nym/pull/965) ([jstuczyn](https://github.com/jstuczyn))
- Additional tauri commands to get bond details [\#964](https://github.com/nymtech/nym/pull/964) ([jstuczyn](https://github.com/jstuczyn))
- Fix topology log [\#962](https://github.com/nymtech/nym/pull/962) ([neacsu](https://github.com/neacsu))
- Network Explorer: configure URLs with `.env` file [\#960](https://github.com/nymtech/nym/pull/960) ([mmsinclair](https://github.com/mmsinclair))
- Add custom denom balance query [\#957](https://github.com/nymtech/nym/pull/957) ([neacsu](https://github.com/neacsu))
- Feature/ts client update [\#956](https://github.com/nymtech/nym/pull/956) ([jstuczyn](https://github.com/jstuczyn))
- Check the response for multiple sends [\#955](https://github.com/nymtech/nym/pull/955) ([neacsu](https://github.com/neacsu))
- Feature/vesting to wallet [\#954](https://github.com/nymtech/nym/pull/954) ([durch](https://github.com/durch))
- Bugfix/rewarding fixes [\#953](https://github.com/nymtech/nym/pull/953) ([jstuczyn](https://github.com/jstuczyn))
- Bump next from 11.1.1 to 11.1.3 in /wallet-web [\#952](https://github.com/nymtech/nym/pull/952) ([dependabot[bot]](https://github.com/apps/dependabot))
- Different workshare calculations for rewarded vs active set [\#951](https://github.com/nymtech/nym/pull/951) ([durch](https://github.com/durch))
- Feature/simulate [\#950](https://github.com/nymtech/nym/pull/950) ([jstuczyn](https://github.com/jstuczyn))
- Feature/profit margin percent config [\#949](https://github.com/nymtech/nym/pull/949) ([durch](https://github.com/durch))
- Run CI for all contracts in one workflow [\#948](https://github.com/nymtech/nym/pull/948) ([durch](https://github.com/durch))
- Desktop Wallet UI Updates [\#947](https://github.com/nymtech/nym/pull/947) ([fmtabbara](https://github.com/fmtabbara))
- Docker updates [\#946](https://github.com/nymtech/nym/pull/946) ([tommyv1987](https://github.com/tommyv1987))
- Add VestingExecute and VestingQuery client traits [\#944](https://github.com/nymtech/nym/pull/944) ([durch](https://github.com/durch))
- Removed reliance on cosmrs fork [\#943](https://github.com/nymtech/nym/pull/943) ([jstuczyn](https://github.com/jstuczyn))
- Feature/terminology update [\#941](https://github.com/nymtech/nym/pull/941) ([jstuczyn](https://github.com/jstuczyn))
- Check the response for other transactions as well [\#937](https://github.com/nymtech/nym/pull/937) ([neacsu](https://github.com/neacsu))
- Allow proxy gateway bonding [\#936](https://github.com/nymtech/nym/pull/936) ([durch](https://github.com/durch))
- Feature/pre cosmrs updates [\#935](https://github.com/nymtech/nym/pull/935) ([jstuczyn](https://github.com/jstuczyn))
- Feature/client on behalf [\#934](https://github.com/nymtech/nym/pull/934) ([neacsu](https://github.com/neacsu))
- Webpack wallet prod configuration [\#933](https://github.com/nymtech/nym/pull/933) ([tommyv1987](https://github.com/tommyv1987))
- Adding tx\_hash to wallet response [\#932](https://github.com/nymtech/nym/pull/932) ([futurechimp](https://github.com/futurechimp))
- Release/1.0.0 pre1 [\#931](https://github.com/nymtech/nym/pull/931) ([durch](https://github.com/durch))
- Feature/identity verification [\#930](https://github.com/nymtech/nym/pull/930) ([jstuczyn](https://github.com/jstuczyn))
- Move cleaned up smart contracts to main code repo  [\#929](https://github.com/nymtech/nym/pull/929) ([mfahampshire](https://github.com/mfahampshire))
- Feature/mixnet contract further adjustments [\#928](https://github.com/nymtech/nym/pull/928) ([jstuczyn](https://github.com/jstuczyn))
- typo copy change for nodemap [\#926](https://github.com/nymtech/nym/pull/926) ([Aid19801](https://github.com/Aid19801))
- Feature/UI enhancements for Desktop Wallet [\#925](https://github.com/nymtech/nym/pull/925) ([fmtabbara](https://github.com/fmtabbara))
- Fixing some clippy warnings [\#922](https://github.com/nymtech/nym/pull/922) ([futurechimp](https://github.com/futurechimp))
- Fixing go warning re unused btc lib [\#921](https://github.com/nymtech/nym/pull/921) ([futurechimp](https://github.com/futurechimp))
- quick fix adding dimensions to nodemap page for consistency [\#920](https://github.com/nymtech/nym/pull/920) ([Aid19801](https://github.com/Aid19801))
- Bump nth-check from 2.0.0 to 2.0.1 in /nym-wallet [\#918](https://github.com/nymtech/nym/pull/918) ([dependabot[bot]](https://github.com/apps/dependabot))
- Fix Mobile View for MUI data-grid \(CARD 108\) [\#915](https://github.com/nymtech/nym/pull/915) ([Aid19801](https://github.com/Aid19801))
- Feature/total delegation bucket [\#913](https://github.com/nymtech/nym/pull/913) ([jstuczyn](https://github.com/jstuczyn))
- Feature/faucet page react [\#911](https://github.com/nymtech/nym/pull/911) ([fmtabbara](https://github.com/fmtabbara))
- Feature/mixnet contract refactor [\#910](https://github.com/nymtech/nym/pull/910) ([futurechimp](https://github.com/futurechimp))
- Update README.md [\#905](https://github.com/nymtech/nym/pull/905) ([tommyv1987](https://github.com/tommyv1987))
- BUG: Bond cell denom  [\#904](https://github.com/nymtech/nym/pull/904) ([Aid19801](https://github.com/Aid19801))
- Explorer UI tests missing data-testid [\#903](https://github.com/nymtech/nym/pull/903) ([tommyv1987](https://github.com/tommyv1987))
- Fix up Nym-Wallet README.md [\#899](https://github.com/nymtech/nym/pull/899) ([tommyv1987](https://github.com/tommyv1987))
- Feature/batch delegator rewarding [\#898](https://github.com/nymtech/nym/pull/898) ([jstuczyn](https://github.com/jstuczyn))
- Bug mapp nodemap [\#897](https://github.com/nymtech/nym/pull/897) ([Aid19801](https://github.com/Aid19801))
- Bug fix/macos keyboard shortcuts [\#896](https://github.com/nymtech/nym/pull/896) ([fmtabbara](https://github.com/fmtabbara))
- Add a Mobile Nav to the Network Explorer [\#895](https://github.com/nymtech/nym/pull/895) ([Aid19801](https://github.com/Aid19801))
- Only use ts-rs in tests [\#894](https://github.com/nymtech/nym/pull/894) ([durch](https://github.com/durch))
- Fix network monitor template [\#892](https://github.com/nymtech/nym/pull/892) ([neacsu](https://github.com/neacsu))
- remove delegation and undelegation from gateways [\#891](https://github.com/nymtech/nym/pull/891) ([fmtabbara](https://github.com/fmtabbara))
- Feature/nym wallet rename [\#890](https://github.com/nymtech/nym/pull/890) ([futurechimp](https://github.com/futurechimp))
- Change MixnodeDetail page's datagrid into a reuseable table component [\#887](https://github.com/nymtech/nym/pull/887) ([Aid19801](https://github.com/Aid19801))
- GitHub Actions: only run job to generate types when not in a PR [\#886](https://github.com/nymtech/nym/pull/886) ([mmsinclair](https://github.com/mmsinclair))
- Adding data-test-ids for the explorer [\#885](https://github.com/nymtech/nym/pull/885) ([tommyv1987](https://github.com/tommyv1987))
- Fix path for github action running tauri-wallet-tests [\#884](https://github.com/nymtech/nym/pull/884) ([tommyv1987](https://github.com/tommyv1987))
- Reverted gateway registration handshake to its 0.11.0 version [\#882](https://github.com/nymtech/nym/pull/882) ([jstuczyn](https://github.com/jstuczyn))
- Network Explorer [\#881](https://github.com/nymtech/nym/pull/881) ([mmsinclair](https://github.com/mmsinclair))
- Feature/rewarding interval updates [\#880](https://github.com/nymtech/nym/pull/880) ([jstuczyn](https://github.com/jstuczyn))
- Put client\_address and id in the correct order [\#875](https://github.com/nymtech/nym/pull/875) ([neacsu](https://github.com/neacsu))
- remove gateway selection on delegation and undelegation pages [\#873](https://github.com/nymtech/nym/pull/873) ([fmtabbara](https://github.com/fmtabbara))
- Set MSRV on all binaries to 1.56 [\#872](https://github.com/nymtech/nym/pull/872) ([jstuczyn](https://github.com/jstuczyn))
- add native window items \(copy/paste\) via tauri [\#871](https://github.com/nymtech/nym/pull/871) ([fmtabbara](https://github.com/fmtabbara))
- Remove stale migration code [\#868](https://github.com/nymtech/nym/pull/868) ([neacsu](https://github.com/neacsu))
- Fixed most recent nightly clippy warnings [\#865](https://github.com/nymtech/nym/pull/865) ([jstuczyn](https://github.com/jstuczyn))
- Active sets =\> Rewarded + Active/Idle sets [\#864](https://github.com/nymtech/nym/pull/864) ([jstuczyn](https://github.com/jstuczyn))
- Chore/cosmrs update [\#862](https://github.com/nymtech/nym/pull/862) ([jstuczyn](https://github.com/jstuczyn))
- Made daily uptime calculation be independent of epoch rewarding [\#860](https://github.com/nymtech/nym/pull/860) ([jstuczyn](https://github.com/jstuczyn))
- Removed epoch rewarding variance [\#857](https://github.com/nymtech/nym/pull/857) ([jstuczyn](https://github.com/jstuczyn))
- Removed gateway rewarding and delegation [\#856](https://github.com/nymtech/nym/pull/856) ([jstuczyn](https://github.com/jstuczyn))
- Update feature-request template [\#855](https://github.com/nymtech/nym/pull/855) ([tommyv1987](https://github.com/tommyv1987))
- Update issue templates [\#854](https://github.com/nymtech/nym/pull/854) ([tommyv1987](https://github.com/tommyv1987))
- Overflow checks in release [\#846](https://github.com/nymtech/nym/pull/846) ([jstuczyn](https://github.com/jstuczyn))
- fix delegate success overflow [\#842](https://github.com/nymtech/nym/pull/842) ([fmtabbara](https://github.com/fmtabbara))
- Feature NYM wallet webdriverio test [\#841](https://github.com/nymtech/nym/pull/841) ([tommyv1987](https://github.com/tommyv1987))
- Update nym\_wallet.yml [\#840](https://github.com/nymtech/nym/pull/840) ([tommyv1987](https://github.com/tommyv1987))
- Feature/vouchers [\#837](https://github.com/nymtech/nym/pull/837) ([aniampio](https://github.com/aniampio))
- Apply readable ids to elements on Nym Wallet [\#836](https://github.com/nymtech/nym/pull/836) ([tommyv1987](https://github.com/tommyv1987))
- Feature/removal of monitor good nodes [\#833](https://github.com/nymtech/nym/pull/833) ([jstuczyn](https://github.com/jstuczyn))
- Feature/bandwidth token [\#832](https://github.com/nymtech/nym/pull/832) ([neacsu](https://github.com/neacsu))
- update app name and icons [\#831](https://github.com/nymtech/nym/pull/831) ([fmtabbara](https://github.com/fmtabbara))
- Create nym-wallet-tests.yml [\#829](https://github.com/nymtech/nym/pull/829) ([tommyv1987](https://github.com/tommyv1987))
- Updated CODEOWNERS [\#828](https://github.com/nymtech/nym/pull/828) ([jstuczyn](https://github.com/jstuczyn))
- Tauri wallet [\#827](https://github.com/nymtech/nym/pull/827) ([fmtabbara](https://github.com/fmtabbara))
- Flag to only run coconut-related functionalities [\#824](https://github.com/nymtech/nym/pull/824) ([jstuczyn](https://github.com/jstuczyn))
- Change false to true, as for mixnodes [\#822](https://github.com/nymtech/nym/pull/822) ([neacsu](https://github.com/neacsu))
- Feature locked client-side bandwidth metering [\#820](https://github.com/nymtech/nym/pull/820) ([jstuczyn](https://github.com/jstuczyn))
- Fixed most recent nightly clippy warnings [\#817](https://github.com/nymtech/nym/pull/817) ([jstuczyn](https://github.com/jstuczyn))
- Feature/resending rewards on timeout [\#810](https://github.com/nymtech/nym/pull/810) ([jstuczyn](https://github.com/jstuczyn))
- Feature/coconut feature [\#805](https://github.com/nymtech/nym/pull/805) ([jstuczyn](https://github.com/jstuczyn))
- Tokenomics rewards [\#802](https://github.com/nymtech/nym/pull/802) ([durch](https://github.com/durch))
- Rocket picking up environment from Rocket.toml again [\#801](https://github.com/nymtech/nym/pull/801) ([jstuczyn](https://github.com/jstuczyn))
- Remove migration code [\#796](https://github.com/nymtech/nym/pull/796) ([neacsu](https://github.com/neacsu))
- Removes code of executed migrations [\#793](https://github.com/nymtech/nym/pull/793) ([jstuczyn](https://github.com/jstuczyn))
- Bugfix/validator api windows build [\#791](https://github.com/nymtech/nym/pull/791) ([jstuczyn](https://github.com/jstuczyn))
- Removed SQLx offline mode artifact [\#790](https://github.com/nymtech/nym/pull/790) ([jstuczyn](https://github.com/jstuczyn))
- Created getters for AccountData [\#787](https://github.com/nymtech/nym/pull/787) ([jstuczyn](https://github.com/jstuczyn))
- Feature/migrate hidden delegations [\#786](https://github.com/nymtech/nym/pull/786) ([neacsu](https://github.com/neacsu))
- Feature/persistent gateway storage [\#784](https://github.com/nymtech/nym/pull/784) ([jstuczyn](https://github.com/jstuczyn))
- Replaced unwrap\_or\_else with unwrap\_or\_default [\#780](https://github.com/nymtech/nym/pull/780) ([jstuczyn](https://github.com/jstuczyn))
- Add block\_height method to Delegation [\#778](https://github.com/nymtech/nym/pull/778) ([durch](https://github.com/durch))
- Make fee helpers public [\#777](https://github.com/nymtech/nym/pull/777) ([durch](https://github.com/durch))
- re-enable bonding [\#776](https://github.com/nymtech/nym/pull/776) ([fmtabbara](https://github.com/fmtabbara))
- Explorer-api: add API resource to show the delegations for each mix node [\#774](https://github.com/nymtech/nym/pull/774) ([mmsinclair](https://github.com/mmsinclair))
- add app alert [\#772](https://github.com/nymtech/nym/pull/772) ([fmtabbara](https://github.com/fmtabbara))
- Migrate legacy delegation data [\#771](https://github.com/nymtech/nym/pull/771) ([durch](https://github.com/durch))
- Adding deps for building the Tauri wallet under Ubuntu [\#770](https://github.com/nymtech/nym/pull/770) ([futurechimp](https://github.com/futurechimp))
- remove alert [\#767](https://github.com/nymtech/nym/pull/767) ([fmtabbara](https://github.com/fmtabbara))
- Feature/consumable bandwidth [\#766](https://github.com/nymtech/nym/pull/766) ([neacsu](https://github.com/neacsu))
- Update coconut-rs and use hash\_to\_scalar from there [\#765](https://github.com/nymtech/nym/pull/765) ([neacsu](https://github.com/neacsu))
- Feature/active sets [\#764](https://github.com/nymtech/nym/pull/764) ([jstuczyn](https://github.com/jstuczyn))
- add app alert banner [\#762](https://github.com/nymtech/nym/pull/762) ([fmtabbara](https://github.com/fmtabbara))
- Updated cosmos-sdk [\#761](https://github.com/nymtech/nym/pull/761) ([jstuczyn](https://github.com/jstuczyn))
- Feature/bond blockstamp [\#760](https://github.com/nymtech/nym/pull/760) ([neacsu](https://github.com/neacsu))
- Feature/revert migration code [\#759](https://github.com/nymtech/nym/pull/759) ([neacsu](https://github.com/neacsu))
- Bump next from 11.1.0 to 11.1.1 in /wallet-web [\#758](https://github.com/nymtech/nym/pull/758) ([dependabot[bot]](https://github.com/apps/dependabot))
- Add block\_height in the Delegation structure as well [\#757](https://github.com/nymtech/nym/pull/757) ([neacsu](https://github.com/neacsu))
- Feature/add blockstamp [\#756](https://github.com/nymtech/nym/pull/756) ([neacsu](https://github.com/neacsu))
- NetworkMonitorBuilder - starting the monitor after rocket has launched [\#754](https://github.com/nymtech/nym/pull/754) ([jstuczyn](https://github.com/jstuczyn))
- Enabled validators api argument [\#753](https://github.com/nymtech/nym/pull/753) ([jstuczyn](https://github.com/jstuczyn))
- Correctly bounding nominator of uptime calculation [\#752](https://github.com/nymtech/nym/pull/752) ([jstuczyn](https://github.com/jstuczyn))
- Fixed argument parsing for ipv6 'good' topology [\#751](https://github.com/nymtech/nym/pull/751) ([jstuczyn](https://github.com/jstuczyn))
- Feature/rust rewarding [\#750](https://github.com/nymtech/nym/pull/750) ([jstuczyn](https://github.com/jstuczyn))
- Revert "Migration commit, will be reverted after the testnet contract… [\#749](https://github.com/nymtech/nym/pull/749) ([neacsu](https://github.com/neacsu))
- Feature/get own delegations [\#748](https://github.com/nymtech/nym/pull/748) ([neacsu](https://github.com/neacsu))
- Feature/more reliable uptime calculation [\#747](https://github.com/nymtech/nym/pull/747) ([jstuczyn](https://github.com/jstuczyn))
- Update template toml key [\#746](https://github.com/nymtech/nym/pull/746) ([neacsu](https://github.com/neacsu))
- Feature/cred after handshake [\#745](https://github.com/nymtech/nym/pull/745) ([neacsu](https://github.com/neacsu))
- Reinstate the POST method blind\_sign [\#744](https://github.com/nymtech/nym/pull/744) ([neacsu](https://github.com/neacsu))
- explorer-api: add pending field to port check response [\#742](https://github.com/nymtech/nym/pull/742) ([mmsinclair](https://github.com/mmsinclair))
- Feature/use delegation rates [\#741](https://github.com/nymtech/nym/pull/741) ([neacsu](https://github.com/neacsu))
- Feature/copy to clipboard [\#740](https://github.com/nymtech/nym/pull/740) ([fmtabbara](https://github.com/fmtabbara))
- Feature/update wallet with stake rates [\#739](https://github.com/nymtech/nym/pull/739) ([neacsu](https://github.com/neacsu))
- Add stake reward rates and bump version of client [\#738](https://github.com/nymtech/nym/pull/738) ([neacsu](https://github.com/neacsu))
- Bump next from 10.1.3 to 11.1.0 in /wallet-web [\#737](https://github.com/nymtech/nym/pull/737) ([dependabot[bot]](https://github.com/apps/dependabot))
- Feature/nymd client integration [\#736](https://github.com/nymtech/nym/pull/736) ([jstuczyn](https://github.com/jstuczyn))
- Bug/fix parking lot on wasm [\#735](https://github.com/nymtech/nym/pull/735) ([neacsu](https://github.com/neacsu))
- Explorer API: add new HTTP resource to decorate mix nodes with geoip locations [\#734](https://github.com/nymtech/nym/pull/734) ([mmsinclair](https://github.com/mmsinclair))
- Feature/completing nymd client api [\#732](https://github.com/nymtech/nym/pull/732) ([jstuczyn](https://github.com/jstuczyn))
- Explorer API - add port check and node description/stats proxy [\#731](https://github.com/nymtech/nym/pull/731) ([mmsinclair](https://github.com/mmsinclair))
- Feature/nymd client fee handling [\#730](https://github.com/nymtech/nym/pull/730) ([jstuczyn](https://github.com/jstuczyn))
- Update DelegationCheck.tsx [\#725](https://github.com/nymtech/nym/pull/725) ([jessgess](https://github.com/jessgess))
- Rust nymd/cosmwasm client [\#724](https://github.com/nymtech/nym/pull/724) ([jstuczyn](https://github.com/jstuczyn))
- Removed wasm feature bypassing cyclic dependencies [\#723](https://github.com/nymtech/nym/pull/723) ([jstuczyn](https://github.com/jstuczyn))
- Updated used sphinx dependency to the most recent revision [\#722](https://github.com/nymtech/nym/pull/722) ([jstuczyn](https://github.com/jstuczyn))
- update state management and validation [\#721](https://github.com/nymtech/nym/pull/721) ([fmtabbara](https://github.com/fmtabbara))
- Add Network Explorer API [\#720](https://github.com/nymtech/nym/pull/720) ([futurechimp](https://github.com/futurechimp))
- Feature/superbuild [\#719](https://github.com/nymtech/nym/pull/719) ([jstuczyn](https://github.com/jstuczyn))
- remove console log [\#718](https://github.com/nymtech/nym/pull/718) ([fmtabbara](https://github.com/fmtabbara))
- Bug/form validation [\#715](https://github.com/nymtech/nym/pull/715) ([fmtabbara](https://github.com/fmtabbara))
- Warnings with identities of good nodes failing checks [\#714](https://github.com/nymtech/nym/pull/714) ([jstuczyn](https://github.com/jstuczyn))
- Removed all sphinx key caching from mixnodes and gateways [\#713](https://github.com/nymtech/nym/pull/713) ([jstuczyn](https://github.com/jstuczyn))
- Feature/receive coins page + UI tweaks [\#704](https://github.com/nymtech/nym/pull/704) ([fmtabbara](https://github.com/fmtabbara))
- Allow users to sign out [\#703](https://github.com/nymtech/nym/pull/703) ([fmtabbara](https://github.com/fmtabbara))
- Feature/docker improvements [\#702](https://github.com/nymtech/nym/pull/702) ([neacsu](https://github.com/neacsu))
- Exposed API port on the validator [\#701](https://github.com/nymtech/nym/pull/701) ([jstuczyn](https://github.com/jstuczyn))
- Feature/default values [\#700](https://github.com/nymtech/nym/pull/700) ([neacsu](https://github.com/neacsu))
- Cleaned up dependencies of our typescript client [\#699](https://github.com/nymtech/nym/pull/699) ([jstuczyn](https://github.com/jstuczyn))
- Bond and delegation alerts [\#698](https://github.com/nymtech/nym/pull/698) ([fmtabbara](https://github.com/fmtabbara))
- Bugfix/network monitor version check [\#697](https://github.com/nymtech/nym/pull/697) ([jstuczyn](https://github.com/jstuczyn))
- Feature/other containers [\#692](https://github.com/nymtech/nym/pull/692) ([neacsu](https://github.com/neacsu))
- Using validator API instead of nymd [\#690](https://github.com/nymtech/nym/pull/690) ([futurechimp](https://github.com/futurechimp))
- Hang coconut issuance off the validator-api [\#679](https://github.com/nymtech/nym/pull/679) ([durch](https://github.com/durch))
- Update hmac and blake3 [\#673](https://github.com/nymtech/nym/pull/673) ([durch](https://github.com/durch))



\* *This Changelog was automatically generated by [github_changelog_generator](https://github.com/github-changelog-generator/github-changelog-generator)*
