# Changelog

## [v0.10.1](https://github.com/nymtech/nym/tree/v0.10.1) (2021-05-24)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.10.0...v0.10.1)

**Closed issues:**

- Prometheus metrics doesn't work [\#606](https://github.com/nymtech/nym/issues/606)
- Bonding hostname vs. ip-address does not show up on NYM explorer [\#593](https://github.com/nymtech/nym/issues/593)
- Cannot assign requested address [\#584](https://github.com/nymtech/nym/issues/584)
- Native client upgrade command is broken [\#582](https://github.com/nymtech/nym/issues/582)
- Spread directory requests across good validators [\#580](https://github.com/nymtech/nym/issues/580)
- Change network monitor to use currency-based rewareds [\#540](https://github.com/nymtech/nym/issues/540)
- Unregistration for protocol ipv6 does not work [\#511](https://github.com/nymtech/nym/issues/511)
- Network monitor view on validators [\#373](https://github.com/nymtech/nym/issues/373)

**Merged pull requests:**

- Bugfix/unique node ownership [\#612](https://github.com/nymtech/nym/pull/612) ([jstuczyn](https://github.com/jstuczyn))
- Decreased log severity for verloc-related functionalities [\#611](https://github.com/nymtech/nym/pull/611) ([jstuczyn](https://github.com/jstuczyn))
- Disabled metrics reporting to the central server [\#609](https://github.com/nymtech/nym/pull/609) ([jstuczyn](https://github.com/jstuczyn))
- Feature/network monitor gateway pings [\#608](https://github.com/nymtech/nym/pull/608) ([jstuczyn](https://github.com/jstuczyn))
- Implemented display traits for identity and encryption keys [\#607](https://github.com/nymtech/nym/pull/607) ([jstuczyn](https://github.com/jstuczyn))
- Feature/add node description api [\#605](https://github.com/nymtech/nym/pull/605) ([futurechimp](https://github.com/futurechimp))
- Feature/updated network monitor [\#604](https://github.com/nymtech/nym/pull/604) ([jstuczyn](https://github.com/jstuczyn))
- Feature/ping timings [\#603](https://github.com/nymtech/nym/pull/603) ([jstuczyn](https://github.com/jstuczyn))
- Bump lodash from 4.17.20 to 4.17.21 in /clients/validator [\#602](https://github.com/nymtech/nym/pull/602) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump lodash from 4.17.20 to 4.17.21 in /clients/native/examples/js-examples/websocket [\#601](https://github.com/nymtech/nym/pull/601) ([dependabot[bot]](https://github.com/apps/dependabot))
- Feature/add rocket [\#600](https://github.com/nymtech/nym/pull/600) ([futurechimp](https://github.com/futurechimp))
- Bump url-parse from 1.4.7 to 1.5.1 in /clients/native/examples/js-examples/websocket [\#599](https://github.com/nymtech/nym/pull/599) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump url-parse from 1.4.7 to 1.5.1 in /clients/webassembly/js-example [\#598](https://github.com/nymtech/nym/pull/598) ([dependabot[bot]](https://github.com/apps/dependabot))
- Recalculating reward rates on appropriate value changes [\#594](https://github.com/nymtech/nym/pull/594) ([jstuczyn](https://github.com/jstuczyn))
- Changed default mixnode query page limit [\#592](https://github.com/nymtech/nym/pull/592) ([jstuczyn](https://github.com/jstuczyn))
- Feature/more exposed client api [\#591](https://github.com/nymtech/nym/pull/591) ([jstuczyn](https://github.com/jstuczyn))
- Contract adjustment to check for node ownership before allowing bonding [\#590](https://github.com/nymtech/nym/pull/590) ([jstuczyn](https://github.com/jstuczyn))
- Bump ssri from 6.0.1 to 6.0.2 in /clients/webassembly/js-example [\#589](https://github.com/nymtech/nym/pull/589) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump ssri from 6.0.1 to 6.0.2 in /clients/native/examples/js-examples/websocket [\#588](https://github.com/nymtech/nym/pull/588) ([dependabot[bot]](https://github.com/apps/dependabot))
- Impl Error trait for ValidatorClientError [\#587](https://github.com/nymtech/nym/pull/587) ([jstuczyn](https://github.com/jstuczyn))
- Checking for tx success when sending coins [\#586](https://github.com/nymtech/nym/pull/586) ([jstuczyn](https://github.com/jstuczyn))
- Logging adjustment [\#585](https://github.com/nymtech/nym/pull/585) ([jstuczyn](https://github.com/jstuczyn))
- Feature/multiple validator endpoints [\#583](https://github.com/nymtech/nym/pull/583) ([jstuczyn](https://github.com/jstuczyn))
- Refreshing nodes gets all available nodes from the contract [\#575](https://github.com/nymtech/nym/pull/575) ([jstuczyn](https://github.com/jstuczyn))
- Feature/simple payments [\#571](https://github.com/nymtech/nym/pull/571) ([jstuczyn](https://github.com/jstuczyn))
- Minor cosmetic changes while reading around [\#568](https://github.com/nymtech/nym/pull/568) ([huitseeker](https://github.com/huitseeker))

## [v0.10.0](https://github.com/nymtech/nym/tree/v0.10.0) (2021-04-15)

[Full Changelog](https://github.com/nymtech/nym/compare/validator-client-0.10.0-rc1...v0.10.0)

This release brings a distributed directory authority powered by [Cosmos SDK](https://cosmos.network) and [CosmWasm](https://cosmwasm.com) smart contracts. It is designed to run [Testnet Finney](https://testnet-finney-explorer.nymtech.net), the new Nym testnet. 



**Closed issues:**

- When I run this command :'./nym-mixnode run --id zzznym', an error occurs [\#548](https://github.com/nymtech/nym/issues/548)

**Merged pull requests:**

- Displaying address of the client on init [\#573](https://github.com/nymtech/nym/pull/573) ([jstuczyn](https://github.com/jstuczyn))
- Fixed nightly clippy warnings [\#572](https://github.com/nymtech/nym/pull/572) ([jstuczyn](https://github.com/jstuczyn))
- Changed default client topology refresh rate from 30s to 5min [\#570](https://github.com/nymtech/nym/pull/570) ([jstuczyn](https://github.com/jstuczyn))
- Adding the wallet url in startup instructions [\#569](https://github.com/nymtech/nym/pull/569) ([futurechimp](https://github.com/futurechimp))
- Removed unused data from cargo.toml [\#567](https://github.com/nymtech/nym/pull/567) ([jstuczyn](https://github.com/jstuczyn))
- Feature/cli signing [\#566](https://github.com/nymtech/nym/pull/566) ([futurechimp](https://github.com/futurechimp))
- Updated version number on the validator client [\#565](https://github.com/nymtech/nym/pull/565) ([jstuczyn](https://github.com/jstuczyn))
- Renamed mixnode registration into bonding [\#564](https://github.com/nymtech/nym/pull/564) ([jstuczyn](https://github.com/jstuczyn))
- A pull request for discussion about contract state variables [\#563](https://github.com/nymtech/nym/pull/563) ([futurechimp](https://github.com/futurechimp))
- Feature/mixnet contract ci [\#562](https://github.com/nymtech/nym/pull/562) ([jstuczyn](https://github.com/jstuczyn))
- Feature/bonding adjustments [\#561](https://github.com/nymtech/nym/pull/561) ([jstuczyn](https://github.com/jstuczyn))
- Feature/migration additions [\#560](https://github.com/nymtech/nym/pull/560) ([jstuczyn](https://github.com/jstuczyn))
- Changed default contract execution gas limit to 250\_000 \(from 9\_000\_000\_000\) [\#559](https://github.com/nymtech/nym/pull/559) ([jstuczyn](https://github.com/jstuczyn))
- Missing quotes in mixnet\_contract\_address config field [\#558](https://github.com/nymtech/nym/pull/558) ([jstuczyn](https://github.com/jstuczyn))
- Updated default validator url and contract address [\#557](https://github.com/nymtech/nym/pull/557) ([jstuczyn](https://github.com/jstuczyn))
- Bump y18n from 4.0.0 to 4.0.1 in /clients/native/examples/js-examples/websocket [\#556](https://github.com/nymtech/nym/pull/556) ([dependabot[bot]](https://github.com/apps/dependabot))
- Feature/bonding info on init [\#555](https://github.com/nymtech/nym/pull/555) ([jstuczyn](https://github.com/jstuczyn))
- Feature/validator client address getter [\#554](https://github.com/nymtech/nym/pull/554) ([jstuczyn](https://github.com/jstuczyn))
- Added extra step for publishing validator client [\#553](https://github.com/nymtech/nym/pull/553) ([jstuczyn](https://github.com/jstuczyn))
- Feature/validator client rc3 [\#552](https://github.com/nymtech/nym/pull/552) ([jstuczyn](https://github.com/jstuczyn))
- Feature/upgrade update [\#551](https://github.com/nymtech/nym/pull/551) ([jstuczyn](https://github.com/jstuczyn))
- Feature/export coin helper [\#550](https://github.com/nymtech/nym/pull/550) ([futurechimp](https://github.com/futurechimp))
- Chore/dependency updates [\#549](https://github.com/nymtech/nym/pull/549) ([jstuczyn](https://github.com/jstuczyn))
- Feature/validator query client [\#547](https://github.com/nymtech/nym/pull/547) ([jstuczyn](https://github.com/jstuczyn))
- Feature/validator client rc2 [\#546](https://github.com/nymtech/nym/pull/546) ([futurechimp](https://github.com/futurechimp))
- Feature/has node query validator client [\#545](https://github.com/nymtech/nym/pull/545) ([jstuczyn](https://github.com/jstuczyn))
- Added contract query to check if given address owns a mixnode/gateway [\#544](https://github.com/nymtech/nym/pull/544) ([jstuczyn](https://github.com/jstuczyn))

## [validator-client-0.10.0-rc1](https://github.com/nymtech/nym/tree/validator-client-0.10.0-rc1) (2021-03-24)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.9.2...validator-client-0.10.0-rc1)

**Implemented enhancements:**

- Add option to whitelist IPv4 ranges to allowed.list in sphinx-socks [\#415](https://github.com/nymtech/nym/issues/415)
- Mixmining monitoring for gateways [\#384](https://github.com/nymtech/nym/issues/384)

**Fixed bugs:**

- Network requester should periodically remove stale proxies [\#424](https://github.com/nymtech/nym/issues/424)
- Network requester now prints correct version with --version [\#478](https://github.com/nymtech/nym/pull/478) ([jstuczyn](https://github.com/jstuczyn))

**Closed issues:**

- Change topology to work with validators. [\#538](https://github.com/nymtech/nym/issues/538)
- Unable to rejoin the network after powerdown [\#514](https://github.com/nymtech/nym/issues/514)
- nym-socks5-client 0.9.2 issue with outbound\_request\_filter.check [\#498](https://github.com/nymtech/nym/issues/498)
- nym-socks5-client high CPU usage on idle [\#491](https://github.com/nymtech/nym/issues/491)
- network requester too many Received a 'Send' before 'Connect' - going to buffer the data [\#483](https://github.com/nymtech/nym/issues/483)
- Socks5 client loops on malformed - invalidaddress message [\#482](https://github.com/nymtech/nym/issues/482)
- Socks5 client hangs [\#479](https://github.com/nymtech/nym/issues/479)
- Network Requester -V flag does not print version [\#469](https://github.com/nymtech/nym/issues/469)
- Gateway reconnection \(wasm\) [\#458](https://github.com/nymtech/nym/issues/458)
- Client warning 'No valid topology' [\#343](https://github.com/nymtech/nym/issues/343)
- private key file permission bits too open, readable for others [\#319](https://github.com/nymtech/nym/issues/319)
- Fix dependabot security notice [\#267](https://github.com/nymtech/nym/issues/267)
- Change how time intervals are serialized in configs [\#141](https://github.com/nymtech/nym/issues/141)

**Merged pull requests:**

- RC1 published [\#543](https://github.com/nymtech/nym/pull/543) ([futurechimp](https://github.com/futurechimp))
- Feature/prep for publish [\#542](https://github.com/nymtech/nym/pull/542) ([futurechimp](https://github.com/futurechimp))
- Feature/bigger better stronger mnemonics [\#541](https://github.com/nymtech/nym/pull/541) ([futurechimp](https://github.com/futurechimp))
- Removed a package-lock.json which seems to have been accidentally added [\#539](https://github.com/nymtech/nym/pull/539) ([futurechimp](https://github.com/futurechimp))
- Feature/convert to uhal [\#537](https://github.com/nymtech/nym/pull/537) ([futurechimp](https://github.com/futurechimp))
- Feature/topology conversion [\#536](https://github.com/nymtech/nym/pull/536) ([jstuczyn](https://github.com/jstuczyn))
- Feature/hook up url in validator client [\#535](https://github.com/nymtech/nym/pull/535) ([futurechimp](https://github.com/futurechimp))
- Feature/upgrade cosm client [\#534](https://github.com/nymtech/nym/pull/534) ([futurechimp](https://github.com/futurechimp))
- Feature/mix contract identity key [\#533](https://github.com/nymtech/nym/pull/533) ([jstuczyn](https://github.com/jstuczyn))
- Feature/validator client rust [\#532](https://github.com/nymtech/nym/pull/532) ([jstuczyn](https://github.com/jstuczyn))
- Feature/add currency helpers [\#531](https://github.com/nymtech/nym/pull/531) ([futurechimp](https://github.com/futurechimp))
- Exporting Coin struct, needed for wallet [\#530](https://github.com/nymtech/nym/pull/530) ([futurechimp](https://github.com/futurechimp))
- Getting correct user home dir in Python setup script [\#528](https://github.com/nymtech/nym/pull/528) ([futurechimp](https://github.com/futurechimp))
- Temporarily disabling fs access [\#527](https://github.com/nymtech/nym/pull/527) ([futurechimp](https://github.com/futurechimp))
- Feature/validator client gateway bonding [\#526](https://github.com/nymtech/nym/pull/526) ([jstuczyn](https://github.com/jstuczyn))
- Fixed eslint errors in the validator client [\#525](https://github.com/nymtech/nym/pull/525) ([jstuczyn](https://github.com/jstuczyn))
- Feature/gateway bonding [\#524](https://github.com/nymtech/nym/pull/524) ([jstuczyn](https://github.com/jstuczyn))
- Fix the remove mixnode test [\#522](https://github.com/nymtech/nym/pull/522) ([futurechimp](https://github.com/futurechimp))
- Bump elliptic from 6.5.3 to 6.5.4 in /clients/native/examples/js-examples/websocket [\#521](https://github.com/nymtech/nym/pull/521) ([dependabot[bot]](https://github.com/apps/dependabot))
- Feature/fix go errors in examples [\#516](https://github.com/nymtech/nym/pull/516) ([futurechimp](https://github.com/futurechimp))
- Feature/initial mixnet contract [\#515](https://github.com/nymtech/nym/pull/515) ([futurechimp](https://github.com/futurechimp))
- Running CI also on windows and macOS [\#512](https://github.com/nymtech/nym/pull/512) ([jstuczyn](https://github.com/jstuczyn))
- Feature/socks5 adjustments [\#510](https://github.com/nymtech/nym/pull/510) ([jstuczyn](https://github.com/jstuczyn))
- Fixed unused import in non-unix systems [\#509](https://github.com/nymtech/nym/pull/509) ([jstuczyn](https://github.com/jstuczyn))
- Checking if the delay has already expired before attempting to put it… [\#508](https://github.com/nymtech/nym/pull/508) ([jstuczyn](https://github.com/jstuczyn))
- Not including gateway non-delay when calculating total packet delay [\#507](https://github.com/nymtech/nym/pull/507) ([jstuczyn](https://github.com/jstuczyn))
- Allowing for a single topology refresh failure [\#505](https://github.com/nymtech/nym/pull/505) ([jstuczyn](https://github.com/jstuczyn))
- More restrictive unix key files permissions [\#504](https://github.com/nymtech/nym/pull/504) ([jstuczyn](https://github.com/jstuczyn))
- More human-readable errors on mixnode/gateway startup [\#503](https://github.com/nymtech/nym/pull/503) ([jstuczyn](https://github.com/jstuczyn))
- Feature/ip filtering [\#502](https://github.com/nymtech/nym/pull/502) ([jstuczyn](https://github.com/jstuczyn))
- Feature/wasm client compilation fixes [\#501](https://github.com/nymtech/nym/pull/501) ([jstuczyn](https://github.com/jstuczyn))
- Fixed possible crash on invalid topology [\#500](https://github.com/nymtech/nym/pull/500) ([jstuczyn](https://github.com/jstuczyn))
- Feature/gateway monitoring [\#499](https://github.com/nymtech/nym/pull/499) ([jstuczyn](https://github.com/jstuczyn))
- Feature/GitHub actions and clippy cleanup [\#493](https://github.com/nymtech/nym/pull/493) ([jstuczyn](https://github.com/jstuczyn))
- Fix typos [\#492](https://github.com/nymtech/nym/pull/492) ([rex4539](https://github.com/rex4539))
- Bump ini from 1.3.5 to 1.3.8 in /clients/native/examples/js-examples/websocket [\#490](https://github.com/nymtech/nym/pull/490) ([dependabot[bot]](https://github.com/apps/dependabot))
- Bump ini from 1.3.5 to 1.3.8 in /clients/webassembly/js-example [\#489](https://github.com/nymtech/nym/pull/489) ([dependabot[bot]](https://github.com/apps/dependabot))
- display 0 on no validators rather than crash [\#488](https://github.com/nymtech/nym/pull/488) ([jstuczyn](https://github.com/jstuczyn))
- NGI0 - Updating licensing aspects according REUSE  [\#487](https://github.com/nymtech/nym/pull/487) ([lnceballosz](https://github.com/lnceballosz))
- Feature/removed topology [\#481](https://github.com/nymtech/nym/pull/481) ([jstuczyn](https://github.com/jstuczyn))
- Bugfix/explorer fixes [\#477](https://github.com/nymtech/nym/pull/477) ([jstuczyn](https://github.com/jstuczyn))
- Feature/0.9.2+only monitoring [\#475](https://github.com/nymtech/nym/pull/475) ([jstuczyn](https://github.com/jstuczyn))

## [v0.9.2](https://github.com/nymtech/nym/tree/v0.9.2) (2020-11-26)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.9.1...v0.9.2)

This release brings networking improvements, eliminating blocking calls and improving mixnode and gateway scalability.

**Fixed bugs:**

- Putting initial packet onto the queue when establishing connection [\#471](https://github.com/nymtech/nym/pull/471) ([jstuczyn](https://github.com/jstuczyn))

**Merged pull requests:**

- Release/v0.9.2 [\#474](https://github.com/nymtech/nym/pull/474) ([jstuczyn](https://github.com/jstuczyn))
- Minor mixnet client code simplification and optimization [\#472](https://github.com/nymtech/nym/pull/472) ([jstuczyn](https://github.com/jstuczyn))

## [v0.9.1](https://github.com/nymtech/nym/tree/v0.9.1) (2020-11-24)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.9.0...v0.9.1)

The main features of this release are:

- explicit `unregister` command for mixnodes
- introduced gateway client reconnection in case of obvious network failures
- changed network monitor to send at a constant, adjustable, rate
- changed the way in which packets are delayed by mixnodes that should reduce number of tasks spawned
- changed the way in which packets are forwarded to further mixes that should get rid of possible blocking

See the changelog for detailed release notes.

**Implemented enhancements:**

- Change how mix packets get delayed [\#361](https://github.com/nymtech/nym/issues/361)
- Feature/socks improvements [\#423](https://github.com/nymtech/nym/pull/423) ([jstuczyn](https://github.com/jstuczyn))
- Feature/instant sending [\#359](https://github.com/nymtech/nym/pull/359) ([jstuczyn](https://github.com/jstuczyn))

**Fixed bugs:**

- Update main.js [\#441](https://github.com/nymtech/nym/pull/441) ([jstuczyn](https://github.com/jstuczyn))
- Bugfix/metrics fixes [\#434](https://github.com/nymtech/nym/pull/434) ([jstuczyn](https://github.com/jstuczyn))
- Bugfix/upgrade fix [\#421](https://github.com/nymtech/nym/pull/421) ([jstuczyn](https://github.com/jstuczyn))
- Explicitly handling base58 key recovery errors [\#396](https://github.com/nymtech/nym/pull/396) ([jstuczyn](https://github.com/jstuczyn))
- Corrected version on client-core [\#377](https://github.com/nymtech/nym/pull/377) ([jstuczyn](https://github.com/jstuczyn))

**Closed issues:**

- Gateway reconnections \(simple\) [\#457](https://github.com/nymtech/nym/issues/457)
- Slow down network monitor sending rate [\#455](https://github.com/nymtech/nym/issues/455)
- Deploy the new explorer on the same box as metrics. [\#433](https://github.com/nymtech/nym/issues/433)
- Too many open files [\#366](https://github.com/nymtech/nym/issues/366)
- nym-mixnode doesn't bind to any port \(Ubuntu 20.04\) [\#290](https://github.com/nymtech/nym/issues/290)

**Merged pull requests:**

- Updated message on shutdown [\#467](https://github.com/nymtech/nym/pull/467) ([jstuczyn](https://github.com/jstuczyn))
- Additional feedback on unregistration on sigint [\#466](https://github.com/nymtech/nym/pull/466) ([jstuczyn](https://github.com/jstuczyn))
- Feature/upgrade additions [\#465](https://github.com/nymtech/nym/pull/465) ([jstuczyn](https://github.com/jstuczyn))
- Feature/nonblocking mix send [\#464](https://github.com/nymtech/nym/pull/464) ([jstuczyn](https://github.com/jstuczyn))
- Feature/delay queue mixnodes [\#462](https://github.com/nymtech/nym/pull/462) ([jstuczyn](https://github.com/jstuczyn))
- Feature/slowed down network monitor [\#461](https://github.com/nymtech/nym/pull/461) ([jstuczyn](https://github.com/jstuczyn))
- Feature/unregister command [\#460](https://github.com/nymtech/nym/pull/460) ([jstuczyn](https://github.com/jstuczyn))
- Simple gateway client reconnection in obvious network failures [\#459](https://github.com/nymtech/nym/pull/459) ([jstuczyn](https://github.com/jstuczyn))
- temporarily disabled mixnode status dot [\#454](https://github.com/nymtech/nym/pull/454) ([jstuczyn](https://github.com/jstuczyn))
- Removed incentives form url [\#451](https://github.com/nymtech/nym/pull/451) ([jstuczyn](https://github.com/jstuczyn))
- Removed hardcoded 'good gateways' in favour of pseusorandom choice fr… [\#450](https://github.com/nymtech/nym/pull/450) ([jstuczyn](https://github.com/jstuczyn))
- Fixed the URL of the websocket [\#449](https://github.com/nymtech/nym/pull/449) ([futurechimp](https://github.com/futurechimp))
- Extra argument to specify metrics websocket + long attribute [\#448](https://github.com/nymtech/nym/pull/448) ([jstuczyn](https://github.com/jstuczyn))
- Explorer public folder being relative to the binary [\#447](https://github.com/nymtech/nym/pull/447) ([jstuczyn](https://github.com/jstuczyn))
- Slightly friendlier upgrade argument description [\#446](https://github.com/nymtech/nym/pull/446) ([jstuczyn](https://github.com/jstuczyn))
- Updated nym-run gateway id [\#445](https://github.com/nymtech/nym/pull/445) ([jstuczyn](https://github.com/jstuczyn))
- Adjusted 'fastmode' settings [\#444](https://github.com/nymtech/nym/pull/444) ([jstuczyn](https://github.com/jstuczyn))
- Added validators to dashboard + validator and block count [\#443](https://github.com/nymtech/nym/pull/443) ([jstuczyn](https://github.com/jstuczyn))
- Adding keybase to allowed.list.sample [\#442](https://github.com/nymtech/nym/pull/442) ([futurechimp](https://github.com/futurechimp))
- Spawning rocket as a blocking task [\#440](https://github.com/nymtech/nym/pull/440) ([jstuczyn](https://github.com/jstuczyn))
- Passing validator base url as an argument [\#439](https://github.com/nymtech/nym/pull/439) ([jstuczyn](https://github.com/jstuczyn))
- Changing default validator location to make it clear we're testnet [\#437](https://github.com/nymtech/nym/pull/437) ([futurechimp](https://github.com/futurechimp))
- Added nym prefix to binary names [\#436](https://github.com/nymtech/nym/pull/436) ([jstuczyn](https://github.com/jstuczyn))
- Feature/explorer [\#431](https://github.com/nymtech/nym/pull/431) ([jstuczyn](https://github.com/jstuczyn))
- Updated default sending rates [\#430](https://github.com/nymtech/nym/pull/430) ([jstuczyn](https://github.com/jstuczyn))
- Fixed bunch of clippy warnings [\#427](https://github.com/nymtech/nym/pull/427) ([jstuczyn](https://github.com/jstuczyn))
- Starting cover traffic stream under correct condition [\#422](https://github.com/nymtech/nym/pull/422) ([jstuczyn](https://github.com/jstuczyn))
- Updated validator topology [\#420](https://github.com/nymtech/nym/pull/420) ([jstuczyn](https://github.com/jstuczyn))
- Added option to set incentives address during mix and gateway init [\#419](https://github.com/nymtech/nym/pull/419) ([jstuczyn](https://github.com/jstuczyn))
- Flag to start network requester in open proxy mode [\#418](https://github.com/nymtech/nym/pull/418) ([jstuczyn](https://github.com/jstuczyn))
- Renamed 'sphinx-socks' to 'network-requester' [\#417](https://github.com/nymtech/nym/pull/417) ([jstuczyn](https://github.com/jstuczyn))
- Updated upgrade commands to set new default validator [\#413](https://github.com/nymtech/nym/pull/413) ([jstuczyn](https://github.com/jstuczyn))
- Feature/network monitor file topology [\#412](https://github.com/nymtech/nym/pull/412) ([jstuczyn](https://github.com/jstuczyn))
- Removed debug print statement [\#411](https://github.com/nymtech/nym/pull/411) ([jstuczyn](https://github.com/jstuczyn))
- Feature/controlled reinit [\#410](https://github.com/nymtech/nym/pull/410) ([jstuczyn](https://github.com/jstuczyn))
- Feature/max retry [\#409](https://github.com/nymtech/nym/pull/409) ([jstuczyn](https://github.com/jstuczyn))
- Renamed directory arguments to validator [\#408](https://github.com/nymtech/nym/pull/408) ([jstuczyn](https://github.com/jstuczyn))
- Feature/no run config flag [\#405](https://github.com/nymtech/nym/pull/405) ([jstuczyn](https://github.com/jstuczyn))
- Feature/error on noninit [\#404](https://github.com/nymtech/nym/pull/404) ([jstuczyn](https://github.com/jstuczyn))
- Using metrics interval received from server [\#403](https://github.com/nymtech/nym/pull/403) ([jstuczyn](https://github.com/jstuczyn))
- Feature/validator api update [\#402](https://github.com/nymtech/nym/pull/402) ([jstuczyn](https://github.com/jstuczyn))
- Feature/directory server transition [\#401](https://github.com/nymtech/nym/pull/401) ([jstuczyn](https://github.com/jstuczyn))
- Feature/wasm client fix [\#399](https://github.com/nymtech/nym/pull/399) ([futurechimp](https://github.com/futurechimp))
- Fix compiler warnings for unneeded mut [\#398](https://github.com/nymtech/nym/pull/398) ([ethanfrey](https://github.com/ethanfrey))
- Feature/fix dependabot alerts [\#393](https://github.com/nymtech/nym/pull/393) ([futurechimp](https://github.com/futurechimp))
- moved new\_v4\_with\_node to test only section [\#392](https://github.com/nymtech/nym/pull/392) ([jstuczyn](https://github.com/jstuczyn))
- Feature/duration cleanup [\#391](https://github.com/nymtech/nym/pull/391) ([jstuczyn](https://github.com/jstuczyn))
- Feature/mix ed25519 identity [\#388](https://github.com/nymtech/nym/pull/388) ([jstuczyn](https://github.com/jstuczyn))
- Feature/double init prevention [\#386](https://github.com/nymtech/nym/pull/386) ([jstuczyn](https://github.com/jstuczyn))
- Feature/upgrade command [\#381](https://github.com/nymtech/nym/pull/381) ([jstuczyn](https://github.com/jstuczyn))
- Feature/remove validator [\#380](https://github.com/nymtech/nym/pull/380) ([futurechimp](https://github.com/futurechimp))
- Feature/version in config [\#376](https://github.com/nymtech/nym/pull/376) ([jstuczyn](https://github.com/jstuczyn))
- Feature/network monitor [\#369](https://github.com/nymtech/nym/pull/369) ([jstuczyn](https://github.com/jstuczyn))
- Added sphinx socks to default workspace members [\#358](https://github.com/nymtech/nym/pull/358) ([jstuczyn](https://github.com/jstuczyn))
- Feature/wasm update [\#341](https://github.com/nymtech/nym/pull/341) ([jstuczyn](https://github.com/jstuczyn))

## [v0.9.0](https://github.com/nymtech/nym/tree/v0.9.0) (2020-11-13)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.8.1...v0.9.0)

The main features of this release are:

* a reputation tracking system which starts to link node reputation to quality of service
* a new component, the `nym-network-monitor`, which tracks whether nodes are working properly and providing good service
* automatic node registration and de-registration at node startup
* working Cosmos validators with a `nym` token
* starting to decentralize the old directory server into the validators
* a new block explorer at https://testnet-explorer.nymtech.net which looks the same as the old dashboard but is the basis of something much more advanced. It can be run by anyone.
* de-coupling metrics collection from directory services to make the system scale better overall
* reliability and performance improvements for mixnode networking

See the changelog for detailed release notes.

**Implemented enhancements:**

- Nicer error if trying to run an uninitialised client/node [\#389](https://github.com/nymtech/nym/issues/389)
- Gateway announcement [\#383](https://github.com/nymtech/nym/issues/383)
- Add init flag for incentives address [\#382](https://github.com/nymtech/nym/issues/382)
- Ed25519 Identity Keys for Mixnodes [\#379](https://github.com/nymtech/nym/issues/379)
- Introduce version field to config files [\#375](https://github.com/nymtech/nym/issues/375)
- Change `init` to not blow away existing keys \(if exist\) [\#368](https://github.com/nymtech/nym/issues/368)
- Introduce an explicit `upgrade` command [\#367](https://github.com/nymtech/nym/issues/367)
- Show remote hostname in socks5 connection messages [\#365](https://github.com/nymtech/nym/issues/365)
- Make all `const` duration values more explicit. [\#333](https://github.com/nymtech/nym/issues/333)

**Fixed bugs:**

- React wasm example not compiling [\#394](https://github.com/nymtech/nym/issues/394)

**Closed issues:**

- Make validator URL configurable [\#438](https://github.com/nymtech/nym/issues/438)
- Change default directory location [\#432](https://github.com/nymtech/nym/issues/432)
- Crank up the default bandwidth settings. [\#429](https://github.com/nymtech/nym/issues/429)
- Change "sphinx-socks" to "nym-requester" [\#428](https://github.com/nymtech/nym/issues/428)
- Clients should use only "active" nodes [\#390](https://github.com/nymtech/nym/issues/390)
- Allow persistently changing config values from command line [\#387](https://github.com/nymtech/nym/issues/387)
- Remove `--config` flag in `run` [\#385](https://github.com/nymtech/nym/issues/385)
- Metrics server should return a metrics rate value [\#374](https://github.com/nymtech/nym/issues/374)
- Integer staking [\#372](https://github.com/nymtech/nym/issues/372)
- Mixnode and gateway blockchain registration [\#371](https://github.com/nymtech/nym/issues/371)
- Remove presence notifications [\#370](https://github.com/nymtech/nym/issues/370)
- Handle invalid base58 encoding for asymmetric key recovery \(encryption and identity\) [\#285](https://github.com/nymtech/nym/issues/285)
- Socks5 nym client + bitcoin service provider [\#254](https://github.com/nymtech/nym/issues/254)
- Message reception in webassembly client [\#204](https://github.com/nymtech/nym/issues/204)
- Simplest possible staking system [\#157](https://github.com/nymtech/nym/issues/157)
- Validator should hold topology [\#77](https://github.com/nymtech/nym/issues/77)

**Merged pull requests:**

- Release/v0.9.0 [\#453](https://github.com/nymtech/nym/pull/453) ([jstuczyn](https://github.com/jstuczyn))

## [v0.8.1](https://github.com/nymtech/nym/tree/v0.8.1) (2020-09-28)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.8.0...v0.8.1)

**Closed issues:**

- Hardcode two gateways for `client init` if none provided [\#352](https://github.com/nymtech/nym/issues/352)
- Make mixnodes take layer with fewest nodes [\#351](https://github.com/nymtech/nym/issues/351)
- Change default presence/metrics interval for mixnodes/gateways [\#349](https://github.com/nymtech/nym/issues/349)
- Mixnodes should only be able to enter layers 1, 2, or 3 [\#348](https://github.com/nymtech/nym/issues/348)
- Docs are inaccurate [\#337](https://github.com/nymtech/nym/issues/337)
- Figure out the cause of high packet loss on testnet [\#159](https://github.com/nymtech/nym/issues/159)
- Change Topology to GraphTopology [\#76](https://github.com/nymtech/nym/issues/76)

**Merged pull requests:**

- Hotfix/0.8.1 [\#357](https://github.com/nymtech/nym/pull/357) ([jstuczyn](https://github.com/jstuczyn))

## [v0.8.0](https://github.com/nymtech/nym/tree/v0.8.0) (2020-09-10)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.7.0...v0.8.0)

This release introduces, among other things, the following improvements:
- SURB-acks for significant boost to the mixnet messaging reliability,
- SURB-replies for allowing for anonymous replies,
- SOCKS5 proxying capabilities,
- replacing the `AuthToken` with a shared key derived between client and its gateway,
- encryption and tagging of mix messages exchanged between client and its gateway,
- end-to-end encryption of traffic between clients,
- general performance and reliability improvements.

**Implemented enhancements:**

- Change how un-ack'd packets are retransmitted [\#307](https://github.com/nymtech/nym/issues/307)
- Feature/socks5 sequencing [\#318](https://github.com/nymtech/nym/pull/318) ([jstuczyn](https://github.com/jstuczyn))
- Feature/socks client config [\#316](https://github.com/nymtech/nym/pull/316) ([jstuczyn](https://github.com/jstuczyn))
- Explicit proxy runner + closing local connection if remote is over [\#314](https://github.com/nymtech/nym/pull/314) ([jstuczyn](https://github.com/jstuczyn))
- Feature/ack messing [\#313](https://github.com/nymtech/nym/pull/313) ([jstuczyn](https://github.com/jstuczyn))
- Removed client list from topology [\#301](https://github.com/nymtech/nym/pull/301) ([jstuczyn](https://github.com/jstuczyn))
- Feature/reply surbs [\#299](https://github.com/nymtech/nym/pull/299) ([jstuczyn](https://github.com/jstuczyn))

**Fixed bugs:**

- Socks client no longer logging dns resolved addresses [\#329](https://github.com/nymtech/nym/pull/329) ([jstuczyn](https://github.com/jstuczyn))
- Bugfix/remove packet buffering [\#300](https://github.com/nymtech/nym/pull/300) ([jstuczyn](https://github.com/jstuczyn))

**Closed issues:**

- Do not buffer packets for mixes we are reconnecting to [\#291](https://github.com/nymtech/nym/issues/291)
- Loop cover messages need to be encrypted! [\#287](https://github.com/nymtech/nym/issues/287)
- Get rid of instances of Deref polymorphism antipattern [\#283](https://github.com/nymtech/nym/issues/283)
- Remove client list from topology [\#279](https://github.com/nymtech/nym/issues/279)
- The messages pushed from gateway should be encrypted. [\#276](https://github.com/nymtech/nym/issues/276)
- The shared key between client and gateway should be stored in a file. [\#273](https://github.com/nymtech/nym/issues/273)
- Refactor topology, NymTopology trait, and related code [\#200](https://github.com/nymtech/nym/issues/200)
- Fragment retransmission for split messages [\#164](https://github.com/nymtech/nym/issues/164)
- Clean up common/clients/mix-clients [\#126](https://github.com/nymtech/nym/issues/126)
- Reliable chunk transmission [\#84](https://github.com/nymtech/nym/issues/84)
- Change how topology is obtained [\#44](https://github.com/nymtech/nym/issues/44)
- More secured auth token - some signature on request [\#5](https://github.com/nymtech/nym/issues/5)

**Merged pull requests:**

- Recommended testnet gateway [\#335](https://github.com/nymtech/nym/pull/335) ([jstuczyn](https://github.com/jstuczyn))
- placeholder contact form url [\#334](https://github.com/nymtech/nym/pull/334) ([jstuczyn](https://github.com/jstuczyn))
- Knocking down delay on message sending default [\#332](https://github.com/nymtech/nym/pull/332) ([futurechimp](https://github.com/futurechimp))
- Made gateway mandatory during init [\#331](https://github.com/nymtech/nym/pull/331) ([jstuczyn](https://github.com/jstuczyn))
- Renaming client binary [\#330](https://github.com/nymtech/nym/pull/330) ([futurechimp](https://github.com/futurechimp))
- v0.8.0 Changelog update [\#328](https://github.com/nymtech/nym/pull/328) ([jstuczyn](https://github.com/jstuczyn))
- Feature/sphinx socks [\#326](https://github.com/nymtech/nym/pull/326) ([futurechimp](https://github.com/futurechimp))
- Feature/print client address on startup [\#325](https://github.com/nymtech/nym/pull/325) ([jstuczyn](https://github.com/jstuczyn))
- Feature/temp wasm example removal [\#324](https://github.com/nymtech/nym/pull/324) ([jstuczyn](https://github.com/jstuczyn))
- Feature/websocket js example dependency update [\#323](https://github.com/nymtech/nym/pull/323) ([jstuczyn](https://github.com/jstuczyn))
- snake\_cased replySURBs [\#322](https://github.com/nymtech/nym/pull/322) ([jstuczyn](https://github.com/jstuczyn))
- Feature/outbound request lists [\#321](https://github.com/nymtech/nym/pull/321) ([futurechimp](https://github.com/futurechimp))
- Feature/temp fix for ipv6 [\#317](https://github.com/nymtech/nym/pull/317) ([futurechimp](https://github.com/futurechimp))
- Removed unused dependencies [\#315](https://github.com/nymtech/nym/pull/315) ([jstuczyn](https://github.com/jstuczyn))
- Feature/perf messing [\#311](https://github.com/nymtech/nym/pull/311) ([futurechimp](https://github.com/futurechimp))
- Upgrades tungstenite libraries to new versions with 64MB message sizes. [\#310](https://github.com/nymtech/nym/pull/310) ([futurechimp](https://github.com/futurechimp))
- Assigning connection shared key post registration [\#308](https://github.com/nymtech/nym/pull/308) ([jstuczyn](https://github.com/jstuczyn))
- Feature/client binary api update [\#306](https://github.com/nymtech/nym/pull/306) ([jstuczyn](https://github.com/jstuczyn))
- Removes unused Cargo dependencies so we stay slim and trim. [\#305](https://github.com/nymtech/nym/pull/305) ([futurechimp](https://github.com/futurechimp))
- Removes unused code from the socks client implementation [\#304](https://github.com/nymtech/nym/pull/304) ([futurechimp](https://github.com/futurechimp))
- Feature/client core [\#303](https://github.com/nymtech/nym/pull/303) ([futurechimp](https://github.com/futurechimp))
- Feature/socks5 [\#302](https://github.com/nymtech/nym/pull/302) ([futurechimp](https://github.com/futurechimp))
- Updated blake3 dependency to 0.3.5 [\#281](https://github.com/nymtech/nym/pull/281) ([jstuczyn](https://github.com/jstuczyn))
- Feature/ws send confirmation removal [\#280](https://github.com/nymtech/nym/pull/280) ([jstuczyn](https://github.com/jstuczyn))
- Added simple react example [\#275](https://github.com/nymtech/nym/pull/275) ([keviinfoes](https://github.com/keviinfoes))
- Feature/topology refactor [\#274](https://github.com/nymtech/nym/pull/274) ([jstuczyn](https://github.com/jstuczyn))
- Feature/gateway shared key generation [\#272](https://github.com/nymtech/nym/pull/272) ([jstuczyn](https://github.com/jstuczyn))
- Removed the healthcheck module, it's no longer in use. [\#271](https://github.com/nymtech/nym/pull/271) ([futurechimp](https://github.com/futurechimp))
- Adding description field to wasm client to kill warning [\#270](https://github.com/nymtech/nym/pull/270) ([futurechimp](https://github.com/futurechimp))
- Running `npm audit fix` on js examples [\#269](https://github.com/nymtech/nym/pull/269) ([futurechimp](https://github.com/futurechimp))
- Feature/constant length packet payloads [\#268](https://github.com/nymtech/nym/pull/268) ([jstuczyn](https://github.com/jstuczyn))
- Feature/wasm topology duplication [\#265](https://github.com/nymtech/nym/pull/265) ([jstuczyn](https://github.com/jstuczyn))
- Removed misplaced WorkingDirectory parameter [\#264](https://github.com/nymtech/nym/pull/264) ([ststefa](https://github.com/ststefa))
- Feature/packet retransmission [\#263](https://github.com/nymtech/nym/pull/263) ([jstuczyn](https://github.com/jstuczyn))
- could not count to ten properly [\#262](https://github.com/nymtech/nym/pull/262) ([ststefa](https://github.com/ststefa))
- build\(deps\): bump websocket-extensions from 0.1.3 to 0.1.4 in /clients/webassembly/js-example [\#261](https://github.com/nymtech/nym/pull/261) ([dependabot[bot]](https://github.com/apps/dependabot))
- add disabling feature 'offline-test' for network-dependent tests [\#260](https://github.com/nymtech/nym/pull/260) ([hyperfekt](https://github.com/hyperfekt))

## [v0.7.0](https://github.com/nymtech/nym/tree/v0.7.0) (2020-06-08)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.6.0...v0.7.0)

The main features of this release are:

* the addition of gateway nodes
* the retiring of the store-and-forward providers in favour of gateway nodes
* got rid of TCP connections for clients, everything now happens through websockets
* a new [Nym webassembly client](https://www.npmjs.com/package/@nymproject/nym-client-wasm), making it possible interact with Nym easily in browser-based runtimes
* reliability and performance improvements for mixnode networking
* initial validator code running (little functionality yet though)

See the [changelog](https://github.com/nymtech/nym/blob/develop/CHANGELOG.md) for detailed release notes. 

**Implemented enhancements:**

- Use tokio codecs for multi\_tcp\_client [\#207](https://github.com/nymtech/nym/issues/207)
- Consider rewriting sfw\_provider\_requests using tokio Framed + Codec [\#181](https://github.com/nymtech/nym/issues/181)

**Fixed bugs:**

- Unexplained traffic increase in presence of unroutable node [\#232](https://github.com/nymtech/nym/issues/232)
- Gateway won't send to restarted layer1 nodes [\#231](https://github.com/nymtech/nym/issues/231)

**Closed issues:**

- Move to userpubkey@gatewaypubkey addresses. [\#235](https://github.com/nymtech/nym/issues/235)
- Get `start_local_network.sh` working with the js example [\#227](https://github.com/nymtech/nym/issues/227)
- Fix indeterminate test failure [\#218](https://github.com/nymtech/nym/issues/218)
- Remove 'fetch' mechanism from desktop client's client in favour of push [\#211](https://github.com/nymtech/nym/issues/211)
- Mixnode - load Sphinx keys like Gateway [\#209](https://github.com/nymtech/nym/issues/209)
- Publish NPM package for WebAssembly client [\#206](https://github.com/nymtech/nym/issues/206)
- Change --sockettype option on desktop client [\#203](https://github.com/nymtech/nym/issues/203)
- Remove TCP sockets from desktop client [\#202](https://github.com/nymtech/nym/issues/202)
- Desktop client currently hard-codes first provider [\#198](https://github.com/nymtech/nym/issues/198)
- Webassembly client currently hard-codes first provider [\#197](https://github.com/nymtech/nym/issues/197)
- Add Rust-based route construction to wasm client [\#196](https://github.com/nymtech/nym/issues/196)
- Remove fetch event [\#195](https://github.com/nymtech/nym/issues/195)
- Control messages should all be JSON [\#194](https://github.com/nymtech/nym/issues/194)
- Desktop Client should attach to gateway websocket [\#193](https://github.com/nymtech/nym/issues/193)
- Merge gateway and provider nodes [\#192](https://github.com/nymtech/nym/issues/192)
- Remove direct Sphinx dependencies [\#184](https://github.com/nymtech/nym/issues/184)
- tests::client\_reconnects\_to\_server\_after\_it\_went\_down fails on aarch64-linux [\#179](https://github.com/nymtech/nym/issues/179)
- \[Windows\] Presence notification fill OS socket queue [\#170](https://github.com/nymtech/nym/issues/170)
- Figure out connection hiccups between client and provider [\#162](https://github.com/nymtech/nym/issues/162)
- Improve the healthchecker [\#160](https://github.com/nymtech/nym/issues/160)
- Rethink client addressability [\#135](https://github.com/nymtech/nym/issues/135)
- Give some love to the service provider client ledger [\#116](https://github.com/nymtech/nym/issues/116)
- Start Gateway node type [\#80](https://github.com/nymtech/nym/issues/80)
- Bring health-checker into validator mix-mining [\#78](https://github.com/nymtech/nym/issues/78)
- Solidify TCPSocket on client [\#72](https://github.com/nymtech/nym/issues/72)
- scripts: run\_local\_network.sh doesn't die nicely [\#45](https://github.com/nymtech/nym/issues/45)
- Persistently store ledger with registered clients and their auth tokens [\#6](https://github.com/nymtech/nym/issues/6)
- Persistent socket connection \(Websocket with client\) [\#17](https://github.com/nymtech/nym/issues/17)
- Persistent socket connection \(TCP Socket with provider\) [\#18](https://github.com/nymtech/nym/issues/18)
- Persistent socket connection \(Websocket with client\) [\#12](https://github.com/nymtech/nym/issues/12)
- Persistent socket connection \(TCP Socket with client\) [\#13](https://github.com/nymtech/nym/issues/13)
- WASM version of the Sphinx packet [\#19](https://github.com/nymtech/nym/issues/19)

**Merged pull requests:**

- Filtering compatible node versions [\#259](https://github.com/nymtech/nym/pull/259) ([jstuczyn](https://github.com/jstuczyn))
- systemd service unit example [\#257](https://github.com/nymtech/nym/pull/257) ([ststefa](https://github.com/ststefa))
- renaming desktop to native client [\#251](https://github.com/nymtech/nym/pull/251) ([futurechimp](https://github.com/futurechimp))
- Adding a pipenv dependencies file to the python client example [\#250](https://github.com/nymtech/nym/pull/250) ([futurechimp](https://github.com/futurechimp))
- Cleaning up startup messages in native client [\#249](https://github.com/nymtech/nym/pull/249) ([futurechimp](https://github.com/futurechimp))
- fixing up readme, bumping version number [\#246](https://github.com/nymtech/nym/pull/246) ([futurechimp](https://github.com/futurechimp))
- Feature/sphinx socket packet encoder [\#245](https://github.com/nymtech/nym/pull/245) ([jstuczyn](https://github.com/jstuczyn))
- Adding some documentation to the webassembly client [\#244](https://github.com/nymtech/nym/pull/244) ([futurechimp](https://github.com/futurechimp))
- Simplified some names and used the published npm package [\#242](https://github.com/nymtech/nym/pull/242) ([futurechimp](https://github.com/futurechimp))
- Feature/make andrew happy [\#241](https://github.com/nymtech/nym/pull/241) ([futurechimp](https://github.com/futurechimp))
- Removed redundant console.log [\#240](https://github.com/nymtech/nym/pull/240) ([jstuczyn](https://github.com/jstuczyn))
- Feature/explicit gateway addressing [\#239](https://github.com/nymtech/nym/pull/239) ([jstuczyn](https://github.com/jstuczyn))
- Feature/clean up [\#238](https://github.com/nymtech/nym/pull/238) ([futurechimp](https://github.com/futurechimp))
- Feature/addressing update [\#237](https://github.com/nymtech/nym/pull/237) ([jstuczyn](https://github.com/jstuczyn))
- Added hidden init flag to increase default traffic volume [\#234](https://github.com/nymtech/nym/pull/234) ([jstuczyn](https://github.com/jstuczyn))
- Bugfix/issue\#231 [\#233](https://github.com/nymtech/nym/pull/233) ([jstuczyn](https://github.com/jstuczyn))
- Fixed unwrap on none value [\#230](https://github.com/nymtech/nym/pull/230) ([jstuczyn](https://github.com/jstuczyn))
- Bugfix/gateway crash on incomplete ws handshake [\#229](https://github.com/nymtech/nym/pull/229) ([jstuczyn](https://github.com/jstuczyn))
- Feature/start local network improvements [\#228](https://github.com/nymtech/nym/pull/228) ([jstuczyn](https://github.com/jstuczyn))
- Updated directory\_client reqwest to 0.10 [\#226](https://github.com/nymtech/nym/pull/226) ([jstuczyn](https://github.com/jstuczyn))
- Updated js-example to get gateway from topology [\#225](https://github.com/nymtech/nym/pull/225) ([jstuczyn](https://github.com/jstuczyn))
- Requiring explicit timestamp when converting from rest to service mix… [\#224](https://github.com/nymtech/nym/pull/224) ([jstuczyn](https://github.com/jstuczyn))
- Feature/minor docs fixes [\#223](https://github.com/nymtech/nym/pull/223) ([futurechimp](https://github.com/futurechimp))
- Removed having to care about SURB\_ID [\#222](https://github.com/nymtech/nym/pull/222) ([jstuczyn](https://github.com/jstuczyn))
- Moved relevant parts of old mix-client to nymsphinx [\#221](https://github.com/nymtech/nym/pull/221) ([jstuczyn](https://github.com/jstuczyn))
- Feature/load keys on run [\#220](https://github.com/nymtech/nym/pull/220) ([jstuczyn](https://github.com/jstuczyn))
- Updated wasm code to work with new gateway and updated the example [\#219](https://github.com/nymtech/nym/pull/219) ([jstuczyn](https://github.com/jstuczyn))
- validator: removing health checker [\#217](https://github.com/nymtech/nym/pull/217) ([futurechimp](https://github.com/futurechimp))
- The great sfw-provider purge of 2020 [\#216](https://github.com/nymtech/nym/pull/216) ([jstuczyn](https://github.com/jstuczyn))
- Fixed compilation warnings on unreachable code when compiling with fe… [\#215](https://github.com/nymtech/nym/pull/215) ([jstuczyn](https://github.com/jstuczyn))
- Feature/healthchecker removal [\#214](https://github.com/nymtech/nym/pull/214) ([jstuczyn](https://github.com/jstuczyn))
- Bugfix/send to correct gateway [\#213](https://github.com/nymtech/nym/pull/213) ([jstuczyn](https://github.com/jstuczyn))
- Feature/client socket adjustments [\#212](https://github.com/nymtech/nym/pull/212) ([jstuczyn](https://github.com/jstuczyn))
- Sending sphinx packet independent of the receiver task [\#210](https://github.com/nymtech/nym/pull/210) ([jstuczyn](https://github.com/jstuczyn))
- Feature/gateway provider merge [\#208](https://github.com/nymtech/nym/pull/208) ([jstuczyn](https://github.com/jstuczyn))
- Feature/route from topology [\#201](https://github.com/nymtech/nym/pull/201) ([futurechimp](https://github.com/futurechimp))
- Intermediate gateway-heart surgery checkpoint [\#199](https://github.com/nymtech/nym/pull/199) ([jstuczyn](https://github.com/jstuczyn))
- Feature/wasm js demo [\#191](https://github.com/nymtech/nym/pull/191) ([futurechimp](https://github.com/futurechimp))
- Feature/improve js example [\#190](https://github.com/nymtech/nym/pull/190) ([futurechimp](https://github.com/futurechimp))
- Feature/limit direct sphinx dependency + remove direct curve25519 dependency from wasm client [\#189](https://github.com/nymtech/nym/pull/189) ([jstuczyn](https://github.com/jstuczyn))
- Feature/very minor refactoring [\#188](https://github.com/nymtech/nym/pull/188) ([jstuczyn](https://github.com/jstuczyn))
- Feature/persistent ledger [\#187](https://github.com/nymtech/nym/pull/187) ([jstuczyn](https://github.com/jstuczyn))
- Optimising wasm build size, shaves about 10% size off our wasm output. [\#186](https://github.com/nymtech/nym/pull/186) ([futurechimp](https://github.com/futurechimp))
- Ran `npm audit fix` on the wasm demo directory. [\#185](https://github.com/nymtech/nym/pull/185) ([futurechimp](https://github.com/futurechimp))
- Feature/nym sphinx wasm [\#183](https://github.com/nymtech/nym/pull/183) ([futurechimp](https://github.com/futurechimp))
- Improvements to sfw-provider - client communcation [\#180](https://github.com/nymtech/nym/pull/180) ([jstuczyn](https://github.com/jstuczyn))
- Adding Apache 2 license headers to all files [\#178](https://github.com/nymtech/nym/pull/178) ([futurechimp](https://github.com/futurechimp))
- Feature/service persistence [\#171](https://github.com/nymtech/nym/pull/171) ([futurechimp](https://github.com/futurechimp))

## [v0.6.0](https://github.com/nymtech/nym/tree/v0.6.0) (2020-04-07)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.5.0...v0.6.0)

This  release fixes bugs in v0.5.0. All testnet node operators are advised to upgrade from v0.5.0.

* fixed premature EOFs on socket connections by using the new multi-TCP client
* fixed a bug causing client and mixnode connection hangs for misconfigured nodes
* by default 'Debug' section of saved configs is now empty and default values are used unless explicitly overridden
* introduced packet chunking allowing clients to send messages of arbitrary length. Note that packet retransmission is not implemented yet, so for longer messages, you might not get anything
* mixnodes now periodically log stats regarding number of packets mixed
* fixed possible client hang ups when sending high rates of traffic 
* preventing mixes from starting with same announce-host as an existing node
* fixed overflow multiplication if connection backoff was set to a high value


**Closed issues:**

- Periodic activity summary [\#172](https://github.com/nymtech/nym/issues/172)
- Move contents of 'common/addressing' into 'common/nymsphinx' [\#161](https://github.com/nymtech/nym/issues/161)
- Make builds simpler for node operators [\#114](https://github.com/nymtech/nym/issues/114)
- Chunking in `nym-client` \(receive\) [\#83](https://github.com/nymtech/nym/issues/83)
- Chunking in `nym-client` \(send\) [\#82](https://github.com/nymtech/nym/issues/82)

**Merged pull requests:**

- Feature/tcp client connection timeout [\#176](https://github.com/nymtech/nym/pull/176) ([jstuczyn](https://github.com/jstuczyn))
- Feature/mixing stats logging [\#175](https://github.com/nymtech/nym/pull/175) ([jstuczyn](https://github.com/jstuczyn))
- Preventing multiplication overflow for reconnection backoff [\#174](https://github.com/nymtech/nym/pull/174) ([jstuczyn](https://github.com/jstuczyn))
- Feature/non mandatory debug config [\#173](https://github.com/nymtech/nym/pull/173) ([jstuczyn](https://github.com/jstuczyn))
- Feature/addressing move [\#169](https://github.com/nymtech/nym/pull/169) ([jstuczyn](https://github.com/jstuczyn))
- Checking if any other node is already announcing the same host [\#168](https://github.com/nymtech/nym/pull/168) ([jstuczyn](https://github.com/jstuczyn))
- Bugfix/closing tcp client connections on drop [\#167](https://github.com/nymtech/nym/pull/167) ([jstuczyn](https://github.com/jstuczyn))
- Yielding tokio task upon creating loop/real traffic message [\#166](https://github.com/nymtech/nym/pull/166) ([jstuczyn](https://github.com/jstuczyn))
- Feature/minor healthchecker improvements [\#165](https://github.com/nymtech/nym/pull/165) ([jstuczyn](https://github.com/jstuczyn))
- Feature/packet chunking [\#158](https://github.com/nymtech/nym/pull/158) ([jstuczyn](https://github.com/jstuczyn))

## [v0.5.0](https://github.com/nymtech/nym/tree/v0.5.0) (2020-03-23)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.5.0-rc.1...v0.5.0)

1. Introduced proper configuration options for mixnodes, clients and providers. Everything is initialised with the `init` command that creates a saved config.toml file. To run the binary you now use `nym-<binary-name> run`, for example `nym-mixnode run`. Each flag can be overwritten at any stage with the following priority: run flags, data in config.toml and finally init flags.
2. Made mixnet TCP connections persistent. When sending a Sphinx packet, it should no longer go through the lengthy process of establishing a TCP connection only to immediately tear it down after sending a single packet. This significantly boosts throughput. 
3. A lot of work on code clean up and refactoring including some performance fixes.
4. Client now determines its default nym-sfw-provider at startup and should always try to connect to the same one. Note: we still can't reliably run more than a single provider on the network.
5. Logging messages now have timestamps and when running at more aggressive log mode (like debug or even trace) we should no longer be overwhelmed with messages from external crates.
6. Initial compatibility with Windows. Please let us know if you have problems.
7. More work on validator, including initial Tendermint integration in Rust, and the start of the mixmining system.

**Closed issues:**

- Introduce timestamps to log messages [\#124](https://github.com/nymtech/nym/issues/124)

**Merged pull requests:**

- removing spooky startup warning message [\#155](https://github.com/nymtech/nym/pull/155) ([futurechimp](https://github.com/futurechimp))
- Some more startup fixes [\#154](https://github.com/nymtech/nym/pull/154) ([futurechimp](https://github.com/futurechimp))
- Entering runtime context when creating mix traffic controller [\#153](https://github.com/nymtech/nym/pull/153) ([jstuczyn](https://github.com/jstuczyn))
- Friendlification of startup messages [\#151](https://github.com/nymtech/nym/pull/151) ([futurechimp](https://github.com/futurechimp))
- Entering runtime context when creating packet forwarder [\#150](https://github.com/nymtech/nym/pull/150) ([jstuczyn](https://github.com/jstuczyn))
- Feature/add topology to validator [\#149](https://github.com/nymtech/nym/pull/149) ([futurechimp](https://github.com/futurechimp))
- Making code work on windows machines [\#148](https://github.com/nymtech/nym/pull/148) ([jstuczyn](https://github.com/jstuczyn))
- validator: adding HTTP interface [\#146](https://github.com/nymtech/nym/pull/146) ([futurechimp](https://github.com/futurechimp))
- Extracting the log setup [\#145](https://github.com/nymtech/nym/pull/145) ([futurechimp](https://github.com/futurechimp))
- Feature/optional location in configs [\#144](https://github.com/nymtech/nym/pull/144) ([jstuczyn](https://github.com/jstuczyn))
- Feature/concurrent connection managers [\#142](https://github.com/nymtech/nym/pull/142) ([jstuczyn](https://github.com/jstuczyn))
- Defaulting for global 'Info' logging level if not set in .env [\#140](https://github.com/nymtech/nym/pull/140) ([jstuczyn](https://github.com/jstuczyn))
- Provider not storing loop cover messages [\#139](https://github.com/nymtech/nym/pull/139) ([jstuczyn](https://github.com/jstuczyn))
- Using log builder to include timestamps + filters [\#138](https://github.com/nymtech/nym/pull/138) ([jstuczyn](https://github.com/jstuczyn))
- Feature/client ws refactoring [\#134](https://github.com/nymtech/nym/pull/134) ([jstuczyn](https://github.com/jstuczyn))
- Bugfix/metrics presence delay fix [\#133](https://github.com/nymtech/nym/pull/133) ([jstuczyn](https://github.com/jstuczyn))
- Removed outdated and redundant sample-configs [\#131](https://github.com/nymtech/nym/pull/131) ([jstuczyn](https://github.com/jstuczyn))
- If not overridden, 'announce-host' should default to 'host' [\#130](https://github.com/nymtech/nym/pull/130) ([jstuczyn](https://github.com/jstuczyn))
- Nice to know who we're talking to at startup... [\#129](https://github.com/nymtech/nym/pull/129) ([futurechimp](https://github.com/futurechimp))

## [v0.5.0-rc.1](https://github.com/nymtech/nym/tree/v0.5.0-rc.1) (2020-03-06)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.4.1...v0.5.0-rc.1)

**Closed issues:**

- COMPILE: Could not compile project using Cargo [\#118](https://github.com/nymtech/nym/issues/118)
- Wherever unbounded mpsc channel is used, prefer unbounded\_send\(\) over send\(\).await [\#90](https://github.com/nymtech/nym/issues/90)
- Add a `Send` method in nym-client [\#81](https://github.com/nymtech/nym/issues/81)
- Start on Tendermint integration [\#79](https://github.com/nymtech/nym/issues/79)
- Ditch DummyKeyPair [\#75](https://github.com/nymtech/nym/issues/75)
- Replace args with proper config files [\#69](https://github.com/nymtech/nym/issues/69)
- Fix incorrectly used Arcs [\#47](https://github.com/nymtech/nym/issues/47)
- nym-mixnode mandatory host option [\#26](https://github.com/nymtech/nym/issues/26)
- Create config struct for mixnode \(possibly also for client\) [\#21](https://github.com/nymtech/nym/issues/21)
- Check if RwLock on MixProcessingData is still needed [\#8](https://github.com/nymtech/nym/issues/8)
- Once implementation is available, wherever appropriate, replace `futures::lock::Mutex` with `futures::lock::RwLock` [\#9](https://github.com/nymtech/nym/issues/9)
- Persistent socket connection with other mixes [\#2](https://github.com/nymtech/nym/issues/2)
- Reuse TCP socket connection between client and mixnodes [\#20](https://github.com/nymtech/nym/issues/20)
- Reuse TCP socket connection between mixnodes and providers [\#3](https://github.com/nymtech/nym/issues/3)

**Merged pull requests:**

- Feature/client refactoring [\#128](https://github.com/nymtech/nym/pull/128) ([jstuczyn](https://github.com/jstuczyn))
- Feature/provider refactoring [\#125](https://github.com/nymtech/nym/pull/125) ([jstuczyn](https://github.com/jstuczyn))
- all: fixing mis-spelling [\#123](https://github.com/nymtech/nym/pull/123) ([futurechimp](https://github.com/futurechimp))
- Feature/further clippy fixes [\#121](https://github.com/nymtech/nym/pull/121) ([jstuczyn](https://github.com/jstuczyn))
- Feature/tokio tungstenite dependency fix [\#120](https://github.com/nymtech/nym/pull/120) ([jstuczyn](https://github.com/jstuczyn))
- Feature/config files cleanup [\#119](https://github.com/nymtech/nym/pull/119) ([jstuczyn](https://github.com/jstuczyn))
- Feature/config files [\#117](https://github.com/nymtech/nym/pull/117) ([jstuczyn](https://github.com/jstuczyn))
- Feature/un genericize keys [\#111](https://github.com/nymtech/nym/pull/111) ([futurechimp](https://github.com/futurechimp))
- Feature/abci [\#110](https://github.com/nymtech/nym/pull/110) ([futurechimp](https://github.com/futurechimp))
- Simplified the use of generics on identity keypair by using output types [\#109](https://github.com/nymtech/nym/pull/109) ([jstuczyn](https://github.com/jstuczyn))

## [v0.4.1](https://github.com/nymtech/nym/tree/v0.4.1) (2020-01-29)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.4.0...v0.4.1)

**Closed issues:**

- Change healthcheck to run on provided topology rather than pull one itself [\#95](https://github.com/nymtech/nym/issues/95)

**Merged pull requests:**

- Bugfix/healthcheck on provided topology [\#108](https://github.com/nymtech/nym/pull/108) ([jstuczyn](https://github.com/jstuczyn))

## [v0.4.0](https://github.com/nymtech/nym/tree/v0.4.0) (2020-01-28)

[Full Changelog](https://github.com/nymtech/nym/compare/0.4.0-rc.2...v0.4.0)

Nym 0.4.0 Platform

In this release, we're taking a lot more care with version numbers, so that we can ensure upgrade compatibility for mixnodes, providers, clients, and validators more easily. 

This release also integrates a health-checker and network topology refresh into the Nym client, so that the client can intelligently choose paths which route around any non-functional or incompatible nodes. 

## [0.4.0-rc.2](https://github.com/nymtech/nym/tree/0.4.0-rc.2) (2020-01-28)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.4.0-rc.2...0.4.0-rc.2)

## [v0.4.0-rc.2](https://github.com/nymtech/nym/tree/v0.4.0-rc.2) (2020-01-28)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.4.0-rc.1...v0.4.0-rc.2)

**Merged pull requests:**

- Hotfix/semver compatibility [\#106](https://github.com/nymtech/nym/pull/106) ([jstuczyn](https://github.com/jstuczyn))

## [v0.4.0-rc.1](https://github.com/nymtech/nym/tree/v0.4.0-rc.1) (2020-01-28)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.3.3...v0.4.0-rc.1)

**Closed issues:**

- Check Sphinx packet length in client [\#98](https://github.com/nymtech/nym/issues/98)
- workflow test [\#97](https://github.com/nymtech/nym/issues/97)
- Client SemVer [\#85](https://github.com/nymtech/nym/issues/85)
- Move PemStore [\#74](https://github.com/nymtech/nym/issues/74)
- Periodic client refresh [\#70](https://github.com/nymtech/nym/issues/70)
- Logging [\#68](https://github.com/nymtech/nym/issues/68)
- Nym-client refactor [\#67](https://github.com/nymtech/nym/issues/67)
- Stop panicking! [\#66](https://github.com/nymtech/nym/issues/66)
- Fix Mixnode Panic on Sphinx packet replay [\#65](https://github.com/nymtech/nym/issues/65)
- Convert older code to start using logging [\#35](https://github.com/nymtech/nym/issues/35)
- Convert to non-url-safe base64 \(everywhere\) [\#28](https://github.com/nymtech/nym/issues/28)
- If a thread blows at startup, panic the entire application [\#15](https://github.com/nymtech/nym/issues/15)
- Split provider/mod.rs [\#10](https://github.com/nymtech/nym/issues/10)

**Merged pull requests:**

- Feature/health checker with existing keys [\#105](https://github.com/nymtech/nym/pull/105) ([jstuczyn](https://github.com/jstuczyn))
- Feature/remove topology equality check [\#104](https://github.com/nymtech/nym/pull/104) ([futurechimp](https://github.com/futurechimp))
- Feature/base58 [\#102](https://github.com/nymtech/nym/pull/102) ([futurechimp](https://github.com/futurechimp))
- Feature/panic improvements [\#101](https://github.com/nymtech/nym/pull/101) ([jstuczyn](https://github.com/jstuczyn))
- Feature/fix sphinx unwraps [\#100](https://github.com/nymtech/nym/pull/100) ([futurechimp](https://github.com/futurechimp))
- Feature/check packet length [\#99](https://github.com/nymtech/nym/pull/99) ([futurechimp](https://github.com/futurechimp))
- Feature/version filtering improvements [\#96](https://github.com/nymtech/nym/pull/96) ([futurechimp](https://github.com/futurechimp))
- Feature/refreshing topology [\#94](https://github.com/nymtech/nym/pull/94) ([jstuczyn](https://github.com/jstuczyn))
- Feature/consistent logging [\#93](https://github.com/nymtech/nym/pull/93) ([futurechimp](https://github.com/futurechimp))
- Feature/semver client [\#92](https://github.com/nymtech/nym/pull/92) ([futurechimp](https://github.com/futurechimp))
- Feature/client refactor [\#91](https://github.com/nymtech/nym/pull/91) ([jstuczyn](https://github.com/jstuczyn))
- Release builds should no longer silently fail - everything will be im… [\#89](https://github.com/nymtech/nym/pull/89) ([jstuczyn](https://github.com/jstuczyn))

## [v0.3.3](https://github.com/nymtech/nym/tree/v0.3.3) (2020-01-20)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.3.2...v0.3.3)

**Fixed bugs:**

- Nym client crashing and disconnecting local websocket with complaint about binary data [\#56](https://github.com/nymtech/nym/issues/56)

**Closed issues:**

- Websocket text fix [\#64](https://github.com/nymtech/nym/issues/64)
- Restore nym-client lib in crate [\#63](https://github.com/nymtech/nym/issues/63)
- Make websocket not crash on ping or pong messages [\#62](https://github.com/nymtech/nym/issues/62)
- Messages returned by fetch are base64 encoded [\#55](https://github.com/nymtech/nym/issues/55)
- Check layer 1 connectivity at client start [\#38](https://github.com/nymtech/nym/issues/38)
- Check required sfw-provider args [\#27](https://github.com/nymtech/nym/issues/27)
- Take version numbers into account when picking routes [\#14](https://github.com/nymtech/nym/issues/14)
- Make Electron app work with new Rust mixnet client [\#16](https://github.com/nymtech/nym/issues/16)

**Merged pull requests:**

- Feature/websocket improvements [\#88](https://github.com/nymtech/nym/pull/88) ([jstuczyn](https://github.com/jstuczyn))
- Using println rather than log for startup banner, it's not an error [\#87](https://github.com/nymtech/nym/pull/87) ([futurechimp](https://github.com/futurechimp))
- Feature/nym client lib [\#61](https://github.com/nymtech/nym/pull/61) ([jstuczyn](https://github.com/jstuczyn))

## [v0.3.2](https://github.com/nymtech/nym/tree/v0.3.2) (2020-01-17)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.3.1...v0.3.2)

**Merged pull requests:**

- Feature/separate presence address [\#59](https://github.com/nymtech/nym/pull/59) ([jstuczyn](https://github.com/jstuczyn))

## [v0.3.1](https://github.com/nymtech/nym/tree/v0.3.1) (2020-01-16)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.3.0...v0.3.1)

**Merged pull requests:**

- Bugfix/presence client crash [\#58](https://github.com/nymtech/nym/pull/58) ([jstuczyn](https://github.com/jstuczyn))

## [v0.3.0](https://github.com/nymtech/nym/tree/v0.3.0) (2020-01-14)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.2.0...v0.3.0)

**Closed issues:**

- Error referring to mismatched types caused by topology [\#46](https://github.com/nymtech/nym/issues/46)
- Provider needs to announce two of its addresses \(+ issue of hardcoded port\) [\#32](https://github.com/nymtech/nym/issues/32)
- Port run\_network.sh from old Go mixnet [\#30](https://github.com/nymtech/nym/issues/30)
- Health Checker inside Validator [\#29](https://github.com/nymtech/nym/issues/29)
- Combine Rust repositories into a single master repo containing multiple crates [\#24](https://github.com/nymtech/nym/issues/24)
- Fix the version numbers on provider and mixnode [\#23](https://github.com/nymtech/nym/issues/23)
- Travis releases for Nym [\#22](https://github.com/nymtech/nym/issues/22)
- Add version number to presence [\#4](https://github.com/nymtech/nym/issues/4)
- Add version number to presence [\#1](https://github.com/nymtech/nym/issues/1)

**Merged pull requests:**

- Feature/client topology filtering [\#54](https://github.com/nymtech/nym/pull/54) ([jstuczyn](https://github.com/jstuczyn))
- print public key for nym client tools [\#53](https://github.com/nymtech/nym/pull/53) ([ghost](https://github.com/ghost))
- Showing binding warning on binding to localhost, 0.0.0.0 or 127.0.0.1 [\#52](https://github.com/nymtech/nym/pull/52) ([jstuczyn](https://github.com/jstuczyn))
- validator: moving sample config files into sample configs directory [\#51](https://github.com/nymtech/nym/pull/51) ([futurechimp](https://github.com/futurechimp))
- Feature/validator health checker [\#50](https://github.com/nymtech/nym/pull/50) ([jstuczyn](https://github.com/jstuczyn))
- Improving websocket connection errors [\#49](https://github.com/nymtech/nym/pull/49) ([futurechimp](https://github.com/futurechimp))
- Feature/crypto crate [\#48](https://github.com/nymtech/nym/pull/48) ([jstuczyn](https://github.com/jstuczyn))
- Feature/chill log messages [\#43](https://github.com/nymtech/nym/pull/43) ([futurechimp](https://github.com/futurechimp))
- persistence: improving PEM file reading and parsing failure messages [\#42](https://github.com/nymtech/nym/pull/42) ([futurechimp](https://github.com/futurechimp))
- Providing a nicer error than "failed on unwrap\(\)" when topology retri… [\#41](https://github.com/nymtech/nym/pull/41) ([futurechimp](https://github.com/futurechimp))
- Prettying up sfw-provider start sequence a bit. [\#40](https://github.com/nymtech/nym/pull/40) ([futurechimp](https://github.com/futurechimp))
- Removing the run command from code and documentation [\#39](https://github.com/nymtech/nym/pull/39) ([futurechimp](https://github.com/futurechimp))
- Feature/script/localnet [\#37](https://github.com/nymtech/nym/pull/37) ([futurechimp](https://github.com/futurechimp))
- Feature/arguments fix [\#36](https://github.com/nymtech/nym/pull/36) ([jstuczyn](https://github.com/jstuczyn))
- Feature/crates cleanup [\#34](https://github.com/nymtech/nym/pull/34) ([jstuczyn](https://github.com/jstuczyn))
- Features/version number to presence [\#25](https://github.com/nymtech/nym/pull/25) ([aniampio](https://github.com/aniampio))
- Feature/minor client changes [\#7](https://github.com/nymtech/nym/pull/7) ([jstuczyn](https://github.com/jstuczyn))

## [v0.2.0](https://github.com/nymtech/nym/tree/v0.2.0) (2020-01-07)

[Full Changelog](https://github.com/nymtech/nym/compare/0.2.0...v0.2.0)

## [0.2.0](https://github.com/nymtech/nym/tree/0.2.0) (2020-01-06)

[Full Changelog](https://github.com/nymtech/nym/compare/3c64a2facd753f4f2f431e7f888e54842e2bc64e...0.2.0)



\* *This Changelog was automatically generated by [github_changelog_generator](https://github.com/github-changelog-generator/github-changelog-generator)*
