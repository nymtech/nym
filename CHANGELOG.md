# Changelog

## [v0.5.0](https://github.com/nymtech/nym/tree/HEAD)

[Full Changelog](https://github.com/nymtech/nym/compare/v0.5.0-rc.1...HEAD)

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
- Reuse TCP socket connection between client and mixnodes [\#20](https://github.com/nymtech/nym/issues/20)
- Once implementation is available, wherever appropriate, replace `futures::lock::Mutex` with `futures::lock::RwLock` [\#9](https://github.com/nymtech/nym/issues/9)
- Check if RwLock on MixProcessingData is still needed [\#8](https://github.com/nymtech/nym/issues/8)
- Reuse TCP socket connection between mixnodes and providers [\#3](https://github.com/nymtech/nym/issues/3)
- Persistent socket connection with other mixes [\#2](https://github.com/nymtech/nym/issues/2)

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
- Make Electron app work with new Rust mixnet client [\#16](https://github.com/nymtech/nym/issues/16)
- Take version numbers into account when picking routes [\#14](https://github.com/nymtech/nym/issues/14)

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
- print public key for nym client tools [\#53](https://github.com/nymtech/nym/pull/53) ([mileschet](https://github.com/mileschet))
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

[Full Changelog](https://github.com/nymtech/nym/compare/0.1.0...0.2.0)



\* *This Changelog was automatically generated by [github_changelog_generator](https://github.com/github-changelog-generator/github-changelog-generator)*
