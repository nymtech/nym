# Changelog

Post 1.0.0 release, the changelog format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [2024.8-wispa] (2024-07-10)

- add event parsing to support cosmos_sdk > 0.50 ([#4697])
- Fix NR config compatibility ([#4690])
- Remove UserAgent constructor since it's weakly typed ([#4689])
- [bugfix]: Node_api_check CLI looked over roles on blacklisted nodes ([#4687])
- Add mixnodes to self describing api cache ([#4684])
- Move and whole bump of crates to workspace and upgrade some ([#4680])
- Remove code that refers to removed nym-network-statistics ([#4679])
- Remove nym-network-statistics ([#4678])
- Create UserAgent that can be passed from the binary to the nym api client ([#4677])
- Add authenticator ([#4667])

[#4697]: https://github.com/nymtech/nym/pull/4697
[#4690]: https://github.com/nymtech/nym/pull/4690
[#4689]: https://github.com/nymtech/nym/pull/4689
[#4687]: https://github.com/nymtech/nym/pull/4687
[#4684]: https://github.com/nymtech/nym/pull/4684
[#4680]: https://github.com/nymtech/nym/pull/4680
[#4679]: https://github.com/nymtech/nym/pull/4679
[#4678]: https://github.com/nymtech/nym/pull/4678
[#4677]: https://github.com/nymtech/nym/pull/4677
[#4667]: https://github.com/nymtech/nym/pull/4667

## [2024.6-chomp] (2024-06-25)

- Remove additional code as part of Ephemera Purge and SP and contracts ([#4650])
- bugfix: make sure nym-api can handle non-cw2 (or without detailed build info) compliant contracts ([#4648])
- introduced a flag to accept toc and exposed it via self-described API ([#4647])
- bugfix: make sure to return an error on invalid public ip ([#4646])
- Add ci check for PR having an assigned milestone ([#4644])
- Removed ephemera code ([#4642])
- Remove stale peers ([#4640])
- Add generic wg private network routing ([#4636])
- Feature/new node endpoints ([#4635])
- standarised ContractBuildInformation and added it to all contracts ([#4631])
- validate nym-node public ips on startup ([#4630])
- Bump defguard wg ([#4625])
- Fix cargo warnings ([#4624])
- Update kernel peers on peer modification ([#4622])
- Handle v6 and v7 requests in the IPR, but reply with v6 ([#4620])
- fix typo ([#4619])
- Update crypto and rand crates ([#4607])
- Purge name service and service provider directory contracts ([#4603])

[#4650]: https://github.com/nymtech/nym/pull/4650
[#4648]: https://github.com/nymtech/nym/pull/4648
[#4647]: https://github.com/nymtech/nym/pull/4647
[#4646]: https://github.com/nymtech/nym/pull/4646
[#4644]: https://github.com/nymtech/nym/pull/4644
[#4642]: https://github.com/nymtech/nym/pull/4642
[#4640]: https://github.com/nymtech/nym/pull/4640
[#4636]: https://github.com/nymtech/nym/pull/4636
[#4635]: https://github.com/nymtech/nym/pull/4635
[#4631]: https://github.com/nymtech/nym/pull/4631
[#4630]: https://github.com/nymtech/nym/pull/4630
[#4625]: https://github.com/nymtech/nym/pull/4625
[#4624]: https://github.com/nymtech/nym/pull/4624
[#4622]: https://github.com/nymtech/nym/pull/4622
[#4620]: https://github.com/nymtech/nym/pull/4620
[#4619]: https://github.com/nymtech/nym/pull/4619
[#4607]: https://github.com/nymtech/nym/pull/4607
[#4603]: https://github.com/nymtech/nym/pull/4603

## [2024.5-ragusa] (2024-05-22)

- Feature/nym node api location ([#4605])
- Add optional signature to IPR request/response ([#4604])
- Feature/unstable tested nodes endpoint ([#4601])
- nym-api: make report/avg_uptime endpoints ignore blacklist ([#4599])
- removed blocking for coconut in the final epoch state ([#4598])
- allow using explicit admin address for issuing freepasses ([#4595])
- Use rfc3339 for last_polled in described nym-api endpoint ([#4591])
- Explicitly handle constraint unique violation when importing credential ([#4588])
- [bugfix] noop flag for nym-api for nymvisor compatibility ([#4586])
- Chore/additional helpers ([#4585])
- Feature/wasm coconut ([#4584])
- upgraded axum and related deps to the most recent version ([#4573])
- Feature/nyxd scraper pruning ([#4564])
- Run cargo autoinherit on the main workspace ([#4553])
- Add rustls-tls to reqwest in validator-client ([#4552])
- Feature/rewarder voucher issuance ([#4548])
- make sure 'OffsetDateTimeJsonSchemaWrapper' is serialised with legacy format  ([#4613])


[#4613]: https://github.com/nymtech/nym/pull/4613
[#4605]: https://github.com/nymtech/nym/pull/4605
[#4604]: https://github.com/nymtech/nym/pull/4604
[#4601]: https://github.com/nymtech/nym/pull/4601
[#4599]: https://github.com/nymtech/nym/pull/4599
[#4598]: https://github.com/nymtech/nym/pull/4598
[#4595]: https://github.com/nymtech/nym/pull/4595
[#4591]: https://github.com/nymtech/nym/pull/4591
[#4588]: https://github.com/nymtech/nym/pull/4588
[#4586]: https://github.com/nymtech/nym/pull/4586
[#4585]: https://github.com/nymtech/nym/pull/4585
[#4584]: https://github.com/nymtech/nym/pull/4584
[#4573]: https://github.com/nymtech/nym/pull/4573
[#4564]: https://github.com/nymtech/nym/pull/4564
[#4553]: https://github.com/nymtech/nym/pull/4553
[#4552]: https://github.com/nymtech/nym/pull/4552
[#4548]: https://github.com/nymtech/nym/pull/4548

## [2024.4-nutella] (2024-05-08)

- [fix] apply disable_poisson_rate from internal NR/IPR cfgs ([#4579])
- updating sign commands to include nym-node ([#4578])
- changed nym-node redirects from 308 'Permanent Redirect' to 303: 'See Other' ([#4572])

[#4579]: https://github.com/nymtech/nym/pull/4579
[#4578]: https://github.com/nymtech/nym/pull/4578
[#4572]: https://github.com/nymtech/nym/pull/4572

## [2024.3-eclipse] (2024-04-22)

- Initial release of the first iteration of the Nym Node
- Improvements to gateway functionality
- IPR development
- Removal of allow list in favour of implementing an exit policy
- Explorer delegation: enables direct delegation to nodes via the Nym Explorer


## [2024.2-fast-and-furious] (2024-03-25)

- Internal testing pre-release 


## [2024.1-marabou] (2024-02-15)

**New Features:**
- Introduced nymvisor support for nym-api, gateway, and mixnode binaries ([#4158])
- Revamped nym-api execution with the addition of init and run commands ([#4225])

**Enhancements:**
- Implemented internal improvements for gateways to optimize internal packet routing
- Improved routing score calculation

**Bug Fixes:**
- Resolved various bugs to enhance overall stability

[#4158]: https://github.com/nymtech/nym/pull/4158
[#4225]: https://github.com/nymtech/nym/pull/4225


## [2023.5-rolo] (2023-11-28)

- Gateway won't open websocket listener until embedded Network Requester becomes available ([#4166])
- Feature/gateway described nr ([#4147])
- Bugfix/prerelease versionbump ([#4145])
- returning 'nil' for non-existing origin as opposed to an empty string ([#4135])
- using performance^20 when calculating active set selection weight ([#4126])
- Change default http API timeout from 3s to 10s ([#4117])

[#4166]: https://github.com/nymtech/nym/issues/4166
[#4147]: https://github.com/nymtech/nym/pull/4147
[#4145]: https://github.com/nymtech/nym/pull/4145
[#4135]: https://github.com/nymtech/nym/pull/4135
[#4126]: https://github.com/nymtech/nym/pull/4126
[#4117]: https://github.com/nymtech/nym/pull/4117

## [2023.nyxd-upgrade] (2023-11-22)

- Chore/nyxd 043 upgrade ([#3968])

[#3968]: https://github.com/nymtech/nym/pull/3968

## [2023.4-galaxy] (2023-11-07)

- DRY up client cli ([#4077])
- [mixnode] replace rocket with axum ([#4071])
- incorporate the nym node HTTP api into the mixnode ([#4070])
- replaced '--disable-sign-ext' with '--signext-lowering' when running wasm-opt ([#3896])
- Added PPA repo hosting support and nym-mixnode package with tooling for publishing ([#4165])

[#4077]: https://github.com/nymtech/nym/pull/4077
[#4071]: https://github.com/nymtech/nym/pull/4071
[#4070]: https://github.com/nymtech/nym/issues/4070
[#3896]: https://github.com/nymtech/nym/pull/3896
[#4165]: https://github.com/nymtech/nym/pull/4165

## [2023.3-kinder] (2023-10-31)

- suppress error output ([#4056])
- Update frontend type for current vesting period ([#4042])
- re-exported additional types for tx queries ([#4036])
- fixed fmt::Display impl for GatewayNetworkRequesterDetails ([#4033])
- Add exit node policy from TorNull and Tor Exit Node Policy ([#4024])
- basic self-described api for gateways to dynamically announce its details + nym-api aggregation ([#4017])
- use saturating sub in case outfox is not enabled ([#3986])
- Fix sorting for mixnodes and gateways ([#3985])
- Gateway client registry and api routes ([#3955])
- Feature/configurable socks5 bind address ([#3992])

[#4056]: https://github.com/nymtech/nym/pull/4056
[#4042]: https://github.com/nymtech/nym/pull/4042
[#4036]: https://github.com/nymtech/nym/pull/4036
[#4033]: https://github.com/nymtech/nym/pull/4033
[#4024]: https://github.com/nymtech/nym/issues/4024
[#4017]: https://github.com/nymtech/nym/issues/4017
[#3986]: https://github.com/nymtech/nym/pull/3986
[#3985]: https://github.com/nymtech/nym/pull/3985
[#3955]: https://github.com/nymtech/nym/pull/3955
[#3992]: https://github.com/nymtech/nym/pull/3992

## [2023.1-milka] (2023-09-24)

- custom Debug impl for mix::Node and gateway::Node ([#3930])
- added forceTls argument to 'MixFetchOptsSimple' ([#3907])
- Enable loop cover traffic by default in NR ([#3904])
- Fix all the cargo warnings ([#3899])
- [Issue] nym-socks5-client crash on UDP request ([#3898])
- Feature/gateway inbuilt nr ([#3877])
- removed queued mixnet migration that was already run ([#3872])
- [feat] Socks5 and Native client: run with hardcoded topology ([#3866])
- Introduce a local network requester directly inside a gateway ([#3838])

[#3930]: https://github.com/nymtech/nym/pull/3930
[#3907]: https://github.com/nymtech/nym/pull/3907
[#3904]: https://github.com/nymtech/nym/pull/3904
[#3899]: https://github.com/nymtech/nym/pull/3899
[#3898]: https://github.com/nymtech/nym/issues/3898
[#3877]: https://github.com/nymtech/nym/pull/3877
[#3872]: https://github.com/nymtech/nym/pull/3872
[#3866]: https://github.com/nymtech/nym/pull/3866
[#3838]: https://github.com/nymtech/nym/issues/3838

## [v1.1.31-kitkat] (2023-09-12)

- feat: add name to `TaskClient` ([#3844])
- added 'open_proxy', 'enabled_statistics' and 'statistics_recipient' to NR config ([#3839])
- MixFetch: initial prototype for insecure HTTP ([#3645])
- MixFetch: prototype implementing TLS in WASM for HTTPS ([#3644])
- SDK: build package for NodeJS ([#3558])
- [Issue] There is already an open connection to this client ([#2845])

[#3844]: https://github.com/nymtech/nym/pull/3844
[#3839]: https://github.com/nymtech/nym/pull/3839
[#3645]: https://github.com/nymtech/nym/issues/3645
[#3644]: https://github.com/nymtech/nym/issues/3644
[#3558]: https://github.com/nymtech/nym/issues/3558
[#2845]: https://github.com/nymtech/nym/issues/2845

## [v1.1.30-twix] (2023-09-05)

- geo_aware_provider: fix too much filtering of gateways ([#3826])
- network-requester: add description to config ([#3799])
- Speedy mode - selects gateway based on latency  in medium / speedy mode ([#3770])
- Chore/enable versioning ([#3768])
- Create explorer-client and use in geo aware provider ([#3824])

[#3826]: https://github.com/nymtech/nym/pull/3826
[#3799]: https://github.com/nymtech/nym/pull/3799
[#3770]: https://github.com/nymtech/nym/issues/3770
[#3768]: https://github.com/nymtech/nym/pull/3768
[#3824]: https://github.com/nymtech/nym/pull/3824

## [v1.1.29-snickers] (2023-08-29)

- Add EXPLORER_API configurable url ([#3810])
- Bugfix/use correct tendermint dialect ([#3802])
- Explorer - look up gateways based on geo-location ([#3776])
- Speedy mode - select the mixnodes based on the location of the NR ([#3775])
- NR - reduce response time by removing poisson delay ([#3774])
- [demo] libp2p example with nym-sdk ([#3763])
- introduced /network/details endpoint to nym-api to return used network information ([#3758])
- Feature/issue credentials ([#3691])

[#3810]: https://github.com/nymtech/nym/pull/3810
[#3802]: https://github.com/nymtech/nym/pull/3802
[#3776]: https://github.com/nymtech/nym/issues/3776
[#3775]: https://github.com/nymtech/nym/issues/3775
[#3774]: https://github.com/nymtech/nym/issues/3774
[#3763]: https://github.com/nymtech/nym/pull/3763
[#3758]: https://github.com/nymtech/nym/pull/3758
[#3691]: https://github.com/nymtech/nym/pull/3691

## [v1.1.28] (2023-08-22)

- [final step3]: add [rust] support to nyxd client in wasm ([#3743])
- Feature/ephemera upgrade ([#3791])
- [rust-sdk] feat: make it more convenient to send and receive messages in different tasks ([#3756])
- feat: validator client refactoring + wasm compatible nyxd client ([#3726])
- feat: retain connection between client init and run ([#3767])

[#3743]: https://github.com/nymtech/nym/issues/3743
[#3791]: https://github.com/nymtech/nym/pull/3791
[#3756]: https://github.com/nymtech/nym/pull/3756
[#3726]: https://github.com/nymtech/nym/pull/3726
[#3767]: https://github.com/nymtech/nym/pull/3767


## [v1.1.27] (2023-08-16)

- fix serialisation of contract types ([#3752])
- Investigate spending credentials from the main API (coconut enabled to a gateway) from feature/ephemera branch ([#3741])
- NymConnect UI stuck in showing "Gateway has issues" ([#3594])
- [UPDATE] Update MiniBolt community-applications-and-guides dev docs ([#3754])

[#3752]: https://github.com/nymtech/nym/issues/3752
[#3741]: https://github.com/nymtech/nym/issues/3741
[#3594]: https://github.com/nymtech/nym/issues/3594
[#3754]: https://github.com/nymtech/nym/pull/3754

## [v1.1.24] (2023-08-08)

- Latency based gateway selection is serial and slow ([#3710])
- Network-requester: strip comments from allow lists ([#3625])
- Remove (or start maintaining) `upgrade` commands from all binaries ([#3600])
- Set sphinx as default packet type ([#3748])
- Apply fix from feature/ephemera to develop too (#3698) ([#3742])
- Feature/coco demos ([#3732])
- Add updates to community list projects ([#3722])
- Add geo-aware mixnet topology provider ([#3713])
- Add updates to community list projects ([#3711])

[#3710]: https://github.com/nymtech/nym/issues/3710
[#3625]: https://github.com/nymtech/nym/issues/3625
[#3600]: https://github.com/nymtech/nym/issues/3600
[#3748]: https://github.com/nymtech/nym/pull/3748
[#3742]: https://github.com/nymtech/nym/pull/3742
[#3732]: https://github.com/nymtech/nym/pull/3732
[#3722]: https://github.com/nymtech/nym/pull/3722
[#3713]: https://github.com/nymtech/nym/pull/3713
[#3711]: https://github.com/nymtech/nym/pull/3711

## [v1.1.23] (2023-07-04)

- nym-cli: add client identity key signing support ([#3576])
- Feature/node tester package ([#3634])
- Add medium toggle to socks5 client ([#3615])
- Don't fully turn off background task when cover traffic is disabled ([#3596])

[#3576]: https://github.com/nymtech/nym/issues/3576
[#3634]: https://github.com/nymtech/nym/pull/3634
[#3615]: https://github.com/nymtech/nym/pull/3615
[#3596]: https://github.com/nymtech/nym/pull/3596

## [v1.1.22] (2023-06-20)

- CLI tool for querying network-requesters ([#3539])
- Statically link OpenSSL ([#3510])
- NymConnect - add sentry.io reporting ([#3421])
- init command does not change version number in config.toml ([#3336])
- [Bug] Config version does not correspond to binary version ([#3434])

[#3539]: https://github.com/nymtech/nym/issues/3539
[#3510]: https://github.com/nymtech/nym/issues/3510
[#3421]: https://github.com/nymtech/nym/issues/3421
[#3336]: https://github.com/nymtech/nym/issues/3336
[#3434]: https://github.com/nymtech/nym/issues/3434

## [v1.1.21] (2023-06-13)

- mixFetch: Change socks5 `SendRequest` to include OrderedMessage index as a field rather than making it serialized inside the `data` field
 ([#3534])
- Explorer - add more data columns to the Service Provider section: ([#3474])
- network-requester: support report if they run an open proxy using `ControlRequest` API ([#3461])
- Refactor client configs (London discussion) ([#3444])
- Increase `DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE` to 2000 to improve reliability ([#3433])
- socks5: sender waits for lanes to clear even though the connection is closed ([#3366])
- version bump for variables ([#3545])

[#3534]: https://github.com/nymtech/nym/issues/3534
[#3474]: https://github.com/nymtech/nym/issues/3474
[#3461]: https://github.com/nymtech/nym/issues/3461
[#3444]: https://github.com/nymtech/nym/issues/3444
[#3433]: https://github.com/nymtech/nym/issues/3433
[#3366]: https://github.com/nymtech/nym/issues/3366
[#3545]: https://github.com/nymtech/nym/pull/3545

## [v1.1.20] (2023-06-06)

- Explorer - Fix SP supported apps list ([#3458])
- Investigate if we need to lower `SHUTDOWN_TIMEOUT` in  socks5 to zero (or almost zero) ([#3438])
- Explorer - show all gateways in the default view regardless of the version number ([#3427])
- service-provider-directory: add signature check when announcing ([#3360])
- Support functionality for nym-name-service (nym-api, nym-cli, etc) ([#3355])
- Edit the nym-network-requester to support the enabled-credentials-mode flag ([#3101])
- [BUG] network requester documentation update ([#3493])
- removing hardcoded version numbers ([#3485])
- [BUG] network requester documentation update ([#3481])
- [BUG] network requester documentation update ([#3469])
- Sign when announcing service providers to the directory contract ([#3459])
- mixnode documentation update ([#3435])
- updated readme with new developer chat links + new docs links ([#3141])

[#3458]: https://github.com/nymtech/nym/issues/3458
[#3438]: https://github.com/nymtech/nym/issues/3438
[#3427]: https://github.com/nymtech/nym/issues/3427
[#3360]: https://github.com/nymtech/nym/issues/3360
[#3355]: https://github.com/nymtech/nym/issues/3355
[#3101]: https://github.com/nymtech/nym/issues/3101
[#3493]: https://github.com/nymtech/nym/pull/3493
[#3485]: https://github.com/nymtech/nym/pull/3485
[#3481]: https://github.com/nymtech/nym/pull/3481
[#3469]: https://github.com/nymtech/nym/pull/3469
[#3459]: https://github.com/nymtech/nym/pull/3459
[#3435]: https://github.com/nymtech/nym/pull/3435
[#3141]: https://github.com/nymtech/nym/pull/3141

## [v1.1.19] (2023-05-16)

- nym-name-service endpoint in nym-api ([#3403])
- Implement key storage for WASM client using IndexedDB (for browser) ([#3329])
- Initial version of nym-name-service contract providing name aliases for nym-addresses ([#3274])
- Update Cargo.lock ([#3410])

[#3403]: https://github.com/nymtech/nym/issues/3403
[#3329]: https://github.com/nymtech/nym/issues/3329
[#3274]: https://github.com/nymtech/nym/issues/3274
[#3410]: https://github.com/nymtech/nym/pull/3410

## [v1.1.18] (2023-05-09)

- Implement heartbeat messages between socks5 proxy and network requester ([#3215])

[#3215]: https://github.com/nymtech/nym/issues/3215

## [v1.1.17] (2023-05-02)

- Add service-provider-directory-contract support to nym-cli ([#3334])
- Start using the node-testing-utils (implemented in #3270) in nym-api Network monitor to simplify the logic there ([#3312])
- Add service-provider-directory support to validator-client ([#3296])
- Allow topology injection in our WASM client ('test my node' feature) ([#3270])
- Expose service-provider-directory contract data in nym-api endpoints ([#3242])
- Cache service provider contract in nym-api ([#3241])
- Feature/1 1 17 docs ([#3370])
- adding a test for SP endpoint ([#3367])
- Feature/store cipher ([#3350])

[#3334]: https://github.com/nymtech/nym/issues/3334
[#3312]: https://github.com/nymtech/nym/issues/3312
[#3296]: https://github.com/nymtech/nym/issues/3296
[#3270]: https://github.com/nymtech/nym/issues/3270
[#3242]: https://github.com/nymtech/nym/issues/3242
[#3241]: https://github.com/nymtech/nym/issues/3241
[#3370]: https://github.com/nymtech/nym/pull/3370
[#3367]: https://github.com/nymtech/nym/pull/3367
[#3350]: https://github.com/nymtech/nym/pull/3350

## [v1.1.16] (2023-04-25)

- Explorer - Fix sorting function on Stake Saturation. It is currently working per page and not globally ([#3320])
- Poisson process gets stuck at too slow rate. Rework to more aggressively up-regulate ([#3309])
- decrease the logging level of warnings associated with clients dropping packets due to gateway being overloaded (I'd say reduce it to debug/trace) - there are few sources of those, e.g. in real and cover traffic streams ([#3299])
- Make the buffer size in `AvailableReader` depend on packet sizes the client is using + introduce read timeouts ([#3213])
- Rust SDK - Support coconut, credential storage etc ([#2755])
- version bump for next release ([#3349])
- added coconut credential generation example ([#3339])
- update mix-node setup docs with node description ([#3325])
- exposed missing gateway commands in nym-cli ([#3324])
- make sure to clear inner 'ack_map' in 'GatewaysReader' ([#3300])

[#3320]: https://github.com/nymtech/nym/issues/3320
[#3309]: https://github.com/nymtech/nym/issues/3309
[#3299]: https://github.com/nymtech/nym/issues/3299
[#3213]: https://github.com/nymtech/nym/issues/3213
[#2755]: https://github.com/nymtech/nym/issues/2755
[#3349]: https://github.com/nymtech/nym/pull/3349
[#3339]: https://github.com/nymtech/nym/pull/3339
[#3325]: https://github.com/nymtech/nym/pull/3325
[#3324]: https://github.com/nymtech/nym/pull/3324
[#3300]: https://github.com/nymtech/nym/pull/3300

## [v1.1.15] (2023-04-18)

- Fix verloc being stuck waiting for shutdown signal ([#3250])
- Introduce `--output json` flag to `sign` command to allow to more easily capture the output ([#3249])
- Explorer - Dont fetch Service Provider list on Testnet ([#3245])
- When determining active set, rather than weighting the nodes by just the `stake`, use `stake * performance` ([#3234])
- Introduce dual packet sizes to our clients (as in use two packet sizes at the same time depending on message size) ([#3189])
- Experiment with offline signing in our validator client ([#3174])
- Modify network requester binary to reload `allowed.list` periodically to pull in any changes made upstream without having to restart the service ([#3149])
- Standardise all `--output json` on binary inits, we pass the output json at different points for different binaries. ([#3080])
- Service provider directory contract: initial version ([#2759])
- Fix issue where network-requester run failed on fresh init due to missing allow file ([#3316])

[#3250]: https://github.com/nymtech/nym/issues/3250
[#3249]: https://github.com/nymtech/nym/issues/3249
[#3245]: https://github.com/nymtech/nym/issues/3245
[#3234]: https://github.com/nymtech/nym/issues/3234
[#3189]: https://github.com/nymtech/nym/issues/3189
[#3174]: https://github.com/nymtech/nym/issues/3174
[#3149]: https://github.com/nymtech/nym/issues/3149
[#3080]: https://github.com/nymtech/nym/issues/3080
[#2759]: https://github.com/nymtech/nym/issues/2759
[#3316]: https://github.com/nymtech/nym/pull/3316

## [v1.1.14] (2023-04-04)

- Investigate cause of qwerty validator being in invalid rewarding state ([#3224])
- Fix NR config due to changes in #3199 ([#3223])
- [Issue] Mixnodes and gateway do not close connections properly  ([#3187])
- disable sign-ext when using wasm-opt + update wasm-opt ([#3203])
- chore: tidy up client `Debug` config section ([#3199])

[#3224]: https://github.com/nymtech/nym/issues/3224
[#3223]: https://github.com/nymtech/nym/issues/3223
[#3187]: https://github.com/nymtech/nym/issues/3187
[#3203]: https://github.com/nymtech/nym/pull/3203
[#3199]: https://github.com/nymtech/nym/pull/3199
>>>>>>> master

## [v1.1.13] (2023-03-15)

- NE - instead of throwing a "Mixnode/Gateway not found" error for blacklisted nodes due to bad performance, show their history but tag them as "Having poor performance" ([#2979])
- NE - Upgrade Sandbox and make below changes:  ([#2332])
- Explorer - Updates ([#3168])
- Website v2 - deploy infrastructure for strapi and CI ([#2213])
- add blockstream green to sp list ([#3180])
- mock-nym-api: fix .storybook lint error ([#3178])
- Validating new interval config parameters to prevent division by zero ([#3153])

[#2979]: https://github.com/nymtech/nym/issues/2979
[#2332]: https://github.com/nymtech/nym/issues/2332
[#3168]: https://github.com/nymtech/nym/issues/3168
[#2213]: https://github.com/nymtech/nym/issues/2213
[#3180]: https://github.com/nymtech/nym/pull/3180
[#3178]: https://github.com/nymtech/nym/pull/3178
[#3153]: https://github.com/nymtech/nym/pull/3153

## [v1.1.12] (2023-03-07)

- Fix generated docs for mixnet and vesting contract on docs.rs ([#3093])
- Introduce a way of injecting topology into the client ([#3044])
- Update mixnet TypeScript client methods #1 ([#2783])
- Update tooltips for routing and average score ([#3133])
- update selected service provider description style ([#3128])

[#3093]: https://github.com/nymtech/nym/issues/3093
[#3044]: https://github.com/nymtech/nym/issues/3044
[#2783]: https://github.com/nymtech/nym/issues/2783
[#3133]: https://github.com/nymtech/nym/pull/3133
[#3128]: https://github.com/nymtech/nym/pull/3128

## [v1.1.11] (2023-02-28)

- Fix empty dealer set loop ([#3105])
- The nym-api db.sqlite is broken when trying to run against it it in `enabled-credentials-mode true` there is an ordering issue with migrations when using the credential binary to purchase bandwidth ([#3100])
- Feature/latency based gateway selection ([#3081])
- Fix the credential binary to handle transactions to sleep when in non-inProgress epochs ([#3057])
- Publish mixnet contract to crates.io ([#1919])
- Publish vesting contract to crates.io ([#1920])
- Feature/update checker to use master ([#3097])
- Feature/improve binary checks ([#3094])

[#3105]: https://github.com/nymtech/nym/issues/3105
[#3100]: https://github.com/nymtech/nym/issues/3100
[#3081]: https://github.com/nymtech/nym/pull/3081
[#3057]: https://github.com/nymtech/nym/issues/3057
[#1919]: https://github.com/nymtech/nym/issues/1919
[#1920]: https://github.com/nymtech/nym/issues/1920
[#3097]: https://github.com/nymtech/nym/pull/3097
[#3094]: https://github.com/nymtech/nym/pull/3094

## [v1.1.10] (2023-02-21)

- Verloc listener causing mixnode unexpected shutdown ([#3038])
- rust-sdk - update API following implementation experience with the network-requester ([#3001])
- Prevent coconut deposits in incompatible states ([#2991])
- Support unavailable signer within threshold ([#2987])
- Implement DKG re-sharing ([#2935])
- contracts: add nym prefix to mixnet and vesting contract packages ([#2855])
- Introduce common interface for all service providers to allow obtaining information such as whether they're online, what binary version they're running, etc. ([#2758])
- Add client functionality to nym-network-requester ([#1900])
- nym-api: uptime rework ([#3053])
- ci: update typescript-lint.yml ([#3035])
- contracts: add nym prefix to mixnet and vesting contract packages ([#2855])

[#3038]: https://github.com/nymtech/nym/issues/3038
[#3001]: https://github.com/nymtech/nym/issues/3001
[#2991]: https://github.com/nymtech/nym/issues/2991
[#2987]: https://github.com/nymtech/nym/issues/2987
[#2935]: https://github.com/nymtech/nym/issues/2935
[#2855]: https://github.com/nymtech/nym/pull/2855
[#2758]: https://github.com/nymtech/nym/issues/2758
[#1900]: https://github.com/nymtech/nym/issues/1900
[#3053]: https://github.com/nymtech/nym/pull/3053
[#3035]: https://github.com/nymtech/nym/pull/3035
[#2855]: https://github.com/nymtech/nym/pull/2855

## [v1.1.9] (2023-02-07)

### Added

- Remove Coconut feature flag ([#2793])
- Separate `nym-api` endpoints with values of "total-supply" and "circulating-supply" in `nym` ([#2964])

### Changed

- native-client: is now capable of listening for requests on sockets different than `127.0.0.1` ([#2912]). This can be specified via `--host` flag during `init` or `run`. Alternatively a custom `host` can be set in `config.toml` file under `socket` section.
- mixnode, gateway: fix unexpected shutdown on corrupted connection ([#2963])

[#2793]: https://github.com/nymtech/nym/issues/2793
[#2912]: https://github.com/nymtech/nym/issues/2912
[#2964]: https://github.com/nymtech/nym/issues/2964
[#2963]: https://github.com/nymtech/nym/issues/3017

## [v1.1.8] (2023-01-31)

### Added

- Rust SDK - Support SURBS (anonymous send + storage) ([#2754])
- dkg rerun from scratch and dkg-specific epochs ([#2810])
- Rename `'initial_supply'` field to `'total_supply'` in the circulating supply endpoint ([#2931])
- Circulating supply api endpoint (read the note inside before testing/deploying) ([#1902])

### Changed

- nym-api: an `--id` flag is now always explicitly required ([#2873])

[#2754]: https://github.com/nymtech/nym/issues/2754
[#2810]: https://github.com/nymtech/nym/issues/2810
[#2931]: https://github.com/nymtech/nym/issues/2931
[#1902]: https://github.com/nymtech/nym/issues/1902
[#2873]: https://github.com/nymtech/nym/issues/2873


## [v1.1.7] (2023-01-24)

### Added

- Gateways now shut down gracefully ([#2019]).
- Rust SDK - Initial version for nym-client ([#2669]).
- Introduce vesting contract query for addresses of all vesting accounts (required for the circulating supply calculation) ([#2778]).
- Add threshold value to the contract storage ([#1893])

### Changed

- Refactor vesting account storage (and in particular, ACCOUNTS saving) ([#2795]).
- Move from manual advancing DKG state to an automatic process ([#2670]).

### Fixed

- Gateways now shut down gracefully ([#2019]).

[#2019]: https://github.com/nymtech/nym/issues/2019
[#2669]: https://github.com/nymtech/nym/issues/2669
[#2795]: https://github.com/nymtech/nym/issues/2795
[#2778]: https://github.com/nymtech/nym/issues/2778
[#2670]: https://github.com/nymtech/nym/issues/2670
[#1893]: https://github.com/nymtech/nym/issues/1893

## [v1.1.6] (2023-01-17)

### Added

- nym-sdk: added initial version of a Rust client sdk
- nym-api: added `/circulating-supply` endpoint ([#2814])
- nym-api: add endpoint listing detailed gateway info by @octol in https://github.com/nymtech/nym/pull/2833

### Changed

- streamline override_config functions -> there's a lot of duplicate if statements everywhere ([#2774])
- clean-up nym-api startup arguments/flags to use clap 3 and its macro-derived arguments ([#2772])
- renamed all references to validator_api to nym_api
- renamed all references to nymd to nyxd ([#2696])
- all-binaries: standarised argument names (note: old names should still be accepted) ([#2762]

### Fixed

- nym-api: should now correctly use `rewarding.enabled` config flag ([#2753])

[#2696]: https://github.com/nymtech/nym/pull/2696
[#2753]: https://github.com/nymtech/nym/pull/2753
[#2762]: https://github.com/nymtech/nym/pull/2762
[#2814]: https://github.com/nymtech/nym/pull/2814
[#2772]: https://github.com/nymtech/nym/pull/2772
[#2774]: https://github.com/nymtech/nym/pull/2774

## [v1.1.5] (2023-01-10)

### Added

- socks5: send status message for service ready, and network-requester error response in https://github.com/nymtech/nym/pull/2715

### Changed

- all-binaries: improved error logging in https://github.com/nymtech/nym/pull/2686
- native client: bring shutdown logic up to the same level as socks5-client in https://github.com/nymtech/nym/pull/2695
- nym-api, coconut-dkg contract: automatic, time-based dkg epoch state advancement in https://github.com/nymtech/nym/pull/2670
- DKG resharing unit test by @neacsu in https://github.com/nymtech/nym/pull/2668
- Renaming validator-api to nym-api by @futurechimp in https://github.com/nymtech/nym/pull/1863
- Modify wasm specific make targets by @neacsu in https://github.com/nymtech/nym/pull/2693
- client: create websocket handler builder by @octol in https://github.com/nymtech/nym/pull/2700
- Outfox and Lion by @durch in https://github.com/nymtech/nym/pull/2730
- Feature/multi surb transmission lanes by @jstuczyn in https://github.com/nymtech/nym/pull/2723

## [v1.1.4] (2022-12-20)

This release adds multiple Single Use Reply Blocks (SURBs) to allow arbitrarily-sized anonymized replies.
At the moment this is turned off by default, but available for use by application developers.
We will need to wait for network-requesters to upgrade to this new release, after which multi-SURB anonymization will become the default setting for the SOCKS proxy clients.

The release also include some additional work for distributed key generation in the Coconut signing authority nodes.

### Changed

- Feature/dkg contract threshold by @neacsu in https://github.com/nymtech/nym/pull/1885
- Multi-surbs by @jstuczyn in https://github.com/nymtech/nym/pull/2667
- Fix multi-surb backwards compatibility in pre 1.1.4 client config files by @jstuczyn in https://github.com/nymtech/nym/pull/2703
- fix: ignore corrupted surb storage and instead create fresh one by @jstuczyn in https://github.com/nymtech/nym/pull/2711
- socks5: rework waiting in inbound.rs by @octol in https://github.com/nymtech/nym/pull/1880

## [v1.1.3] (2022-12-13)

### Changed

- validator-api: can recover from shutdown during DKG process ([#1872])
- clients: deduplicate gateway initialization, part of work towards a rust-sdk
- clients: keep all transmission lanes going at all times by making priority probabilistic
- clients: ability to use multi-reply SURBs to send arbitrarily long messages fully anonymously whilst requesting additional reply blocks whenever they're about to run out ([#1796], [#1801], [#1804], [#1835], [#1858], [#1883]))

### Fixed

- network-requester: fix bug where websocket connection disconnect resulted in success error code
- clients: fix a few panics handling the gateway-client
- mixnode, gateway, validator-api: Use mainnet values as defaults for URLs and mixnet contract ([#1884])
- socks5: fixed bug where connections sometimes where closed too early
- clients: improve message logging when received message fails to get reconstructed ([#1803])

[#1796]: https://github.com/nymtech/nym/pull/1796
[#1801]: https://github.com/nymtech/nym/pull/1801
[#1803]: https://github.com/nymtech/nym/pull/1803
[#1804]: https://github.com/nymtech/nym/pull/1804
[#1835]: https://github.com/nymtech/nym/pull/1835
[#1858]: https://github.com/nymtech/nym/pull/1858
[#1872]: https://github.com/nymtech/nym/pull/1872
[#1883]: https://github.com/nymtech/nym/pull/1883
[#1884]: https://github.com/nymtech/nym/pull/1884

## [v1.1.2]

### Changed

- gateway: Renamed flag from `enabled/disabled_credentials_mode` to `only-coconut-credentials`
- "Family" feature for node families + layers
- Initial coconut functionality including credentials and distributed key generation

## [v1.1.1](https://github.com/nymtech/nym/tree/v1.1.1) (2022-11-29)

### Added

- binaries: add `-c` shortform for `--config-env-file`
- websocket-requests: add server response signalling current packet queue length in the client
- contracts: DKG contract that handles coconut key generation ([#1678][#1708][#1747])
- validator-api: generate coconut keys interactively, using DKG and multisig contracts ([#1678][#1708][#1747])

### Changed

- clients: add concept of transmission lanes to better handle multiple data streams ([#1720])
- clients,validator-api: take coconut signers from the chain instead of specifying them via CLI ([#1747])
- multisig contract: add DKG contract to the list of addresses that can create proposals ([#1747])
- socks5-client: wait closing inbound connection until data is sent, and throttle incoming data in general ([#1783])
- nym-cli: improve error reporting/handling and changed `vesting-schedule` queries to use query client instead of signing client

### Fixed

- gateway-client: fix decrypting stored messages on reconnect ([#1786])

### Fixed

- gateway-client: fix decrypting stored messages on reconnect ([#1786])
- socks5-client: fix shutting down all tasks if anyone of them panics or errors out ([#1805])

[#1678]: https://github.com/nymtech/nym/pull/1678
[#1708]: https://github.com/nymtech/nym/pull/1708
[#1720]: https://github.com/nymtech/nym/pull/1720
[#1747]: https://github.com/nymtech/nym/pull/1747
[#1783]: https://github.com/nymtech/nym/pull/1783
[#1786]: https://github.com/nymtech/nym/pull/1786
[#1805]: https://github.com/nymtech/nym/pull/1805

## [v1.1.0](https://github.com/nymtech/nym/tree/v1.1.0) (2022-11-09)

### Added

- clients: add testing-only support for two more extended packet sizes (8kb and 16kb).
- common/ledger: new library for communicating with a Ledger device ([#1640])
- native-client/socks5-client/wasm-client: `disable_loop_cover_traffic_stream` Debug config option to disable the separate loop cover traffic stream ([#1666])
- native-client/socks5-client/wasm-client: `disable_main_poisson_packet_distribution` Debug config option to make the client ignore poisson distribution in the main packet stream and ONLY send real message (and as fast as they come) ([#1664])
- native-client/socks5-client/wasm-client: `use_extended_packet_size` Debug config option to make the client use 'ExtendedPacketSize' for its traffic (32kB as opposed to 2kB in 1.0.2) ([#1671])
- network-requester: added additional Blockstream Green wallet endpoint to `example.allowed.list` ([#1611])
- validator-api: add `interval_operating_cost` and `profit_margin_percent` to compute reward estimation endpoint
- validator-client: added `query_contract_smart` and `query_contract_raw` on `NyxdClient` ([#1558])
- wasm-client: uses updated wasm-compatible `client-core` so that it's now capable of packet retransmission, cover traffic and poisson delay (among other things!) ([#1673])

### Fixed

- socks5-client: fix bug where in some cases packet reordering could trigger a connection being closed too early ([#1702],[#1724])
- validator-api: mixnode, gateway should now prefer values in config.toml over mainnet defaults ([#1645])
- validator-api: should now correctly update historical uptimes for all mixnodes and gateways every 24h ([#1721])

### Changed

- clients: bound the sphinx packet channel and reduce sending rate if gateway can't keep up ([#1703],[#1725])
- gateway-client: will attempt to read now as many as 8 websocket messages at once, assuming they're already available on the socket ([#1669])
- moved `Percent` struct to `contracts-common`, change affects explorer-api
- socks5 client: graceful shutdown should fix error on disconnect in nym-connect ([#1591])
- validator-api: changed error serialization on `inclusion_probability`, `stake-saturation` and `reward-estimation` endpoints to provide more accurate information ([#1681])
- validator-client: made `fee` argument optional for `execute` and `execute_multiple` ([#1541])
- wasm-client: fixed build errors on MacOS and changed example JS code to use mainnet ([#1585])
- validator-api: changes to internal SQL schema due to the mixnet contract revamp ([#1472])
- validator-api: changes to internal data structures due to the mixnet contract revamp ([#1472])
- validator-api: split epoch-operations into multiple separate transactions ([#1472])

[#1472]: https://github.com/nymtech/nym/pull/1472
[#1541]: https://github.com/nymtech/nym/pull/1541
[#1558]: https://github.com/nymtech/nym/pull/1558
[#1577]: https://github.com/nymtech/nym/pull/1577
[#1585]: https://github.com/nymtech/nym/pull/1585
[#1591]: https://github.com/nymtech/nym/pull/1591
[#1640]: https://github.com/nymtech/nym/pull/1640
[#1645]: https://github.com/nymtech/nym/pull/1645
[#1611]: https://github.com/nymtech/nym/pull/1611
[#1664]: https://github.com/nymtech/nym/pull/1664
[#1666]: https://github.com/nymtech/nym/pull/1645
[#1669]: https://github.com/nymtech/nym/pull/1669
[#1671]: https://github.com/nymtech/nym/pull/1671
[#1673]: https://github.com/nymtech/nym/pull/1673
[#1681]: https://github.com/nymtech/nym/pull/1681
[#1702]: https://github.com/nymtech/nym/pull/1702
[#1703]: https://github.com/nymtech/nym/pull/1703
[#1721]: https://github.com/nymtech/nym/pull/1721
[#1724]: https://github.com/nymtech/nym/pull/1724
[#1725]: https://github.com/nymtech/nym/pull/1725

## [nym-binaries-1.0.2](https://github.com/nymtech/nym/tree/nym-binaries-1.0.2)

### Added

- socks5 client/websocket client: add `--force-register-gateway` flag, useful when rerunning init ([#1353])
- all: added network compilation target to `--help` (or `--version`) commands ([#1256]).
- explorer-api: learned how to sum the delegations by owner in a new endpoint.
- explorer-api: add apy values to `mix_nodes` endpoint
- gateway: Added gateway coconut verifications and validator-api communication for double spending protection ([#1261])
- network-explorer-ui: Upgrade to React Router 6
- rewarding: replace circulating supply with staking supply in reward calculations ([#1324])
- validator-api: add `estimated_node_profit` and `estimated_operator_cost` to `reward-estimate` endpoint ([#1284])
- validator-api: add detailed mixnode bond endpoints, and explorer-api makes use of that data to append stake saturation
- validator-api: add Swagger to document the REST API ([#1249]).
- validator-api: Added new endpoints for coconut spending flow and communications with coconut & multisig contracts ([#1261])
- validator-api: add `uptime`, `estimated_operator_apy`, `estimated_delegators_apy` to `/mixnodes/detailed` endpoint ([#1393])
- validator-api: add node info cache storing simulated active set inclusion probabilities
- network-statistics: a new mixnet service that aggregates and exposes anonymized data about mixnet services ([#1328])
- mixnode: Added basic mixnode hardware reporting to the HTTP API ([#1308]).
- validator-api: endpoint, in coconut mode, for returning the validator-api cosmos address ([#1404]).
- validator-client: add `denom` argument and add simple test for querying an account balance
- gateway, validator-api: Checks for coconut credential double spending attempts, taking the coconut bandwidth contract as source of truth ([#1457])
- coconut-bandwidth-contract: Record the state of a coconut credential; create specific proposal for releasing funds ([#1457])
- inclusion-probability: add simulator for active set inclusion probability

### Fixed

- mixnode, gateway: attempting to determine reconnection backoff to persistently failing mixnode could result in a crash ([#1260])
- mixnode: the mixnode learned how to shutdown gracefully
- mixnode: listen out for SIGTERM and SIGQUIT too, making it play nicely as a system service.
- native & socks5 clients: fail early when clients try to re-init with a different gateway, which is not supported yet ([#1322])
- native & socks5 clients: rerun init will now reuse previous gateway configuration instead of failing ([#1353])
- native & socks5 clients: deduplicate big chunks of init logic
- validator: fixed local docker-compose setup to work on Apple M1 ([#1329])
- explorer-api: listen out for SIGTERM and SIGQUIT too, making it play nicely as a system service ([#1482]).
- network-requester: fix filter for suffix-only domains ([#1487])
- validator-api: listen out for SIGTERM and SIGQUIT too, making it play nicely as a system service; cleaner shutdown, without panics ([#1496], [#1573]).

### Changed

- validator-client: created internal `Coin` type that replaces coins from `cosmrs` and `cosmwasm` for API entrypoints [[#1295]]
- all: updated all `cosmwasm`-related dependencies to `1.0.0` and `cw-storage-plus` to `0.13.4` [[#1318]]
- all: updated `rocket` to `0.5.0-rc.2`.
- network-requester: allow to voluntarily store and send statistical data about the number of bytes the proxied server serves ([#1328])
- gateway: allow to voluntarily send statistical data about the number of active inboxes served by a gateway ([#1376])
- gateway & mixnode: move detailed build info back to `--version` from `--help`.
- socks5 client/websocket client: upgrade to latest clap and switched to declarative commandline parsing.
- validator-api: fee payment for multisig operations comes from the gateway account instead of the validator APIs' accounts ([#1419])
- multisig-contract: Limit the proposal creating functionality to one address (coconut-bandwidth-contract address) ([#1457])
- All binaries and cosmwasm blobs are configured at runtime now; binaries are configured using environment variables or .env files and contracts keep the configuration parameters in storage ([#1463])
- gateway, network-statistics: include gateway id in the sent statistical data ([#1478])
- network explorer: tweak how active set probability is shown ([#1503])
- validator-api: rewarder set update fails without panicking on possible nyxd queries ([#1520])
- network-requester, socks5 client (nym-connect): send and receive respectively a message error to be displayed about filter check failure ([#1576])

[#1249]: https://github.com/nymtech/nym/pull/1249
[#1256]: https://github.com/nymtech/nym/pull/1256
[#1260]: https://github.com/nymtech/nym/pull/1260
[#1261]: https://github.com/nymtech/nym/pull/1261
[#1267]: https://github.com/nymtech/nym/pull/1267
[#1278]: https://github.com/nymtech/nym/pull/1278
[#1295]: https://github.com/nymtech/nym/pull/1295
[#1302]: https://github.com/nymtech/nym/pull/1302
[#1308]: https://github.com/nymtech/nym/pull/1308
[#1318]: https://github.com/nymtech/nym/pull/1318
[#1322]: https://github.com/nymtech/nym/pull/1322
[#1324]: https://github.com/nymtech/nym/pull/1324
[#1328]: https://github.com/nymtech/nym/pull/1328
[#1329]: https://github.com/nymtech/nym/pull/1329
[#1353]: https://github.com/nymtech/nym/pull/1353
[#1376]: https://github.com/nymtech/nym/pull/1376
[#1393]: https://github.com/nymtech/nym/pull/1393
[#1404]: https://github.com/nymtech/nym/pull/1404
[#1419]: https://github.com/nymtech/nym/pull/1419
[#1457]: https://github.com/nymtech/nym/pull/1457
[#1463]: https://github.com/nymtech/nym/pull/1463
[#1478]: https://github.com/nymtech/nym/pull/1478
[#1482]: https://github.com/nymtech/nym/pull/1482
[#1487]: https://github.com/nymtech/nym/pull/1487
[#1496]: https://github.com/nymtech/nym/pull/1496
[#1503]: https://github.com/nymtech/nym/pull/1503
[#1520]: https://github.com/nymtech/nym/pull/1520
[#1573]: https://github.com/nymtech/nym/pull/1573
[#1576]: https://github.com/nymtech/nym/pull/1576

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

## [nym-binaries-1.0.0-rc.2](https://github.com/nymtech/nym/tree/nym-binaries-1.0.0-rc.2) (2022-04-15)

[Full Changelog](https://github.com/nymtech/nym/compare/nym-wallet-v1.0.2...nym-binaries-1.0.0-rc.2)

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
- Bump minimist from 1.2.5 to 1.2.6 in /docker/typescript_client/upload_contract [\#1159](https://github.com/nymtech/nym/pull/1159) ([dependabot[bot]](https://github.com/apps/dependabot))
- Feature/vesting full [\#1158](https://github.com/nymtech/nym/pull/1158) ([fmtabbara](https://github.com/fmtabbara))
- get_current_epoch tauri [\#1156](https://github.com/nymtech/nym/pull/1156) ([durch](https://github.com/durch))
- Cleanup [\#1155](https://github.com/nymtech/nym/pull/1155) ([durch](https://github.com/durch))
- Feature flag reward payments [\#1154](https://github.com/nymtech/nym/pull/1154) ([durch](https://github.com/durch))
- Add Query endpoints for calculating rewards [\#1152](https://github.com/nymtech/nym/pull/1152) ([durch](https://github.com/durch))
- Pending endpoints [\#1150](https://github.com/nymtech/nym/pull/1150) ([durch](https://github.com/durch))
- wallet: add logging [\#1149](https://github.com/nymtech/nym/pull/1149) ([octol](https://github.com/octol))
- wallet: use Urls rather than Strings for validator urls [\#1148](https://github.com/nymtech/nym/pull/1148) ([octol](https://github.com/octol))
- Change accumulated reward to Option, migrate delegations [\#1147](https://github.com/nymtech/nym/pull/1147) ([durch](https://github.com/durch))
- wallet: fetch validators url remotely if available [\#1146](https://github.com/nymtech/nym/pull/1146) ([octol](https://github.com/octol))
- Fix delegated_free calculation [\#1145](https://github.com/nymtech/nym/pull/1145) ([durch](https://github.com/durch))
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

## [v0.12.1](https://github.com/nymtech/nym/tree/v0.12.1) (2021-12-23)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.12.0...v0.12.1)

**Implemented enhancements:**

- Add version check to binaries [\#967](https://github.com/nymtech/nym/issues/967)

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
- Bug fix: Network Explorer: Add freegeoip API key and split out tasks for country distributions [\#806](https://github.com/nymtech/nym/pull/806) ([mmsinclair](https://github.com/mmsinclair))
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
- got sign issue from bot [\#709](https://github.com/nymtech/nym/issues/709)
- As a wallet user, I would like to be able to log out of the wallet [\#706](https://github.com/nymtech/nym/issues/706)
- As a wallet user, I would like to have a "receive" page where I can see my own wallet address [\#705](https://github.com/nymtech/nym/issues/705)
- Update native client/socks client/mixnode/gateway `upgrade` command [\#689](https://github.com/nymtech/nym/issues/689)
- Update mixnode/gateway/client to use query for cached nodes rather than use validator [\#688](https://github.com/nymtech/nym/issues/688)
- '--directory' not expected error starting local mixnet [\#520](https://github.com/nymtech/nym/issues/520)
- nym-socks5-client is painfully slow [\#495](https://github.com/nymtech/nym/issues/495)
- nym-socks5-client crash after opening Keybase team "Browse all channels" [\#494](https://github.com/nymtech/nym/issues/494)
- Mixed Content problem [\#400](https://github.com/nymtech/nym/issues/400)
- Gateway disk quota [\#137](https://github.com/nymtech/nym/issues/137)
- Simplify message encapsulation with regards to topology [\#127](https://github.com/nymtech/nym/issues/127)
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
- Adding tx_hash to wallet response [\#932](https://github.com/nymtech/nym/pull/932) ([futurechimp](https://github.com/futurechimp))
- Release/1.0.0 pre1 [\#931](https://github.com/nymtech/nym/pull/931) ([durch](https://github.com/durch))
- Feature/identity verification [\#930](https://github.com/nymtech/nym/pull/930) ([jstuczyn](https://github.com/jstuczyn))
- Move cleaned up smart contracts to main code repo [\#929](https://github.com/nymtech/nym/pull/929) ([mfahampshire](https://github.com/mfahampshire))
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
- BUG: Bond cell denom [\#904](https://github.com/nymtech/nym/pull/904) ([Aid19801](https://github.com/Aid19801))
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
- Put client_address and id in the correct order [\#875](https://github.com/nymtech/nym/pull/875) ([neacsu](https://github.com/neacsu))
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
- Update nym_wallet.yml [\#840](https://github.com/nymtech/nym/pull/840) ([tommyv1987](https://github.com/tommyv1987))
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
- Replaced unwrap_or_else with unwrap_or_default [\#780](https://github.com/nymtech/nym/pull/780) ([jstuczyn](https://github.com/jstuczyn))
- Add block_height method to Delegation [\#778](https://github.com/nymtech/nym/pull/778) ([durch](https://github.com/durch))
- Make fee helpers public [\#777](https://github.com/nymtech/nym/pull/777) ([durch](https://github.com/durch))
- re-enable bonding [\#776](https://github.com/nymtech/nym/pull/776) ([fmtabbara](https://github.com/fmtabbara))
- Explorer-api: add API resource to show the delegations for each mix node [\#774](https://github.com/nymtech/nym/pull/774) ([mmsinclair](https://github.com/mmsinclair))
- add app alert [\#772](https://github.com/nymtech/nym/pull/772) ([fmtabbara](https://github.com/fmtabbara))
- Migrate legacy delegation data [\#771](https://github.com/nymtech/nym/pull/771) ([durch](https://github.com/durch))
- Adding deps for building the Tauri wallet under Ubuntu [\#770](https://github.com/nymtech/nym/pull/770) ([futurechimp](https://github.com/futurechimp))
- remove alert [\#767](https://github.com/nymtech/nym/pull/767) ([fmtabbara](https://github.com/fmtabbara))
- Feature/consumable bandwidth [\#766](https://github.com/nymtech/nym/pull/766) ([neacsu](https://github.com/neacsu))
- Update coconut-rs and use hash_to_scalar from there [\#765](https://github.com/nymtech/nym/pull/765) ([neacsu](https://github.com/neacsu))
- Feature/active sets [\#764](https://github.com/nymtech/nym/pull/764) ([jstuczyn](https://github.com/jstuczyn))
- add app alert banner [\#762](https://github.com/nymtech/nym/pull/762) ([fmtabbara](https://github.com/fmtabbara))
- Updated cosmos-sdk [\#761](https://github.com/nymtech/nym/pull/761) ([jstuczyn](https://github.com/jstuczyn))
- Feature/bond blockstamp [\#760](https://github.com/nymtech/nym/pull/760) ([neacsu](https://github.com/neacsu))
- Feature/revert migration code [\#759](https://github.com/nymtech/nym/pull/759) ([neacsu](https://github.com/neacsu))
- Bump next from 11.1.0 to 11.1.1 in /wallet-web [\#758](https://github.com/nymtech/nym/pull/758) ([dependabot[bot]](https://github.com/apps/dependabot))
- Add block_height in the Delegation structure as well [\#757](https://github.com/nymtech/nym/pull/757) ([neacsu](https://github.com/neacsu))
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
- Reinstate the POST method blind_sign [\#744](https://github.com/nymtech/nym/pull/744) ([neacsu](https://github.com/neacsu))
- explorer-api: add pending field to port check response [\#742](https://github.com/nymtech/nym/pull/742) ([mmsinclair](https://github.com/mmsinclair))
- Feature/use delegation rates [\#741](https://github.com/nymtech/nym/pull/741) ([neacsu](https://github.com/neacsu))
- Feature/copy to clipboard [\#740](https://github.com/nymtech/nym/pull/740) ([fmtabbara](https://github.com/fmtabbara))
- Feature/update wallet with stake rates [\#739](https://github.com/nymtech/nym/pull/739) ([neacsu](https://github.com/neacsu))
- Add stake reward rates and bump version of client [\#738](https://github.com/nymtech/nym/pull/738) ([neacsu](https://github.com/neacsu))
- Bump next from 10.1.3 to 11.1.0 in /wallet-web [\#737](https://github.com/nymtech/nym/pull/737) ([dependabot[bot]](https://github.com/apps/dependabot))
- Feature/nyxd client integration [\#736](https://github.com/nymtech/nym/pull/736) ([jstuczyn](https://github.com/jstuczyn))
- Bug/fix parking lot on wasm [\#735](https://github.com/nymtech/nym/pull/735) ([neacsu](https://github.com/neacsu))
- Explorer API: add new HTTP resource to decorate mix nodes with geoip locations [\#734](https://github.com/nymtech/nym/pull/734) ([mmsinclair](https://github.com/mmsinclair))
- Feature/completing nyxd client api [\#732](https://github.com/nymtech/nym/pull/732) ([jstuczyn](https://github.com/jstuczyn))
- Explorer API - add port check and node description/stats proxy [\#731](https://github.com/nymtech/nym/pull/731) ([mmsinclair](https://github.com/mmsinclair))
- Feature/nyxd client fee handling [\#730](https://github.com/nymtech/nym/pull/730) ([jstuczyn](https://github.com/jstuczyn))
- Update DelegationCheck.tsx [\#725](https://github.com/nymtech/nym/pull/725) ([jessgess](https://github.com/jessgess))
- Rust nyxd/cosmwasm client [\#724](https://github.com/nymtech/nym/pull/724) ([jstuczyn](https://github.com/jstuczyn))
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
- Using validator API instead of nyxd [\#690](https://github.com/nymtech/nym/pull/690) ([futurechimp](https://github.com/futurechimp))
- Hang coconut issuance off the validator-api [\#679](https://github.com/nymtech/nym/pull/679) ([durch](https://github.com/durch))
- Update hmac and blake3 [\#673](https://github.com/nymtech/nym/pull/673) ([durch](https://github.com/durch))

\* _This Changelog was automatically generated by [github_changelog_generator](https://github.com/github-changelog-generator/github-changelog-generator)_
