# Route log

1. --list (given in prompt)
2. GUESS search "what is Nym mixnet overview introduction" -> /docs/operators/introduction (wrong audience), /docs/network, /docs/network/mixnet-mode
3. GUESS search "Rust SDK integrate application with mixnet" -> /docs/developers/rust, /docs/developers (crate table)
4. LINK get_section /docs/developers -> front door: two-axis (runtime x e2e/proxy), points to choose-config, swizzle, limitations
5. GUESS search "choose a defence which configuration threat model" -> surfaced /network/threat-model/examples/wallet (jackpot, not linked from anywhere I'd been)
6. LINK get_section wallet page -> INTRO ONLY (bare URL trap)
7. GUESS search "wallet sync lightwalletd adversary..." -> anchors for wallet page + swizzle-zcash
8. GUESS search "limitations bandwidth sending rate..." -> limitations anchors
9. LINK get_section /developers/swizzle -> intro (+ leaked maintenance comment)
10. GUESS search "smolmix TcpStream over mixnet" -> smolmix page anchors
11. GUESS search "exit policy allowed ports blocked IPR NR" -> exit-services trust model; NO clean port list; operator iptables noise
12. GUESS search "bandwidth credentials ticketbook zk-nym" -> zk-nym pages; KEY: "SDK integrations currently connect to the Mixnet without requiring credentials"; smoldvpn DOES need ticketbooks
13. GUESS "rotate exit per request random IPR client pool" -> threat-model/configurations/mixnet-rotating, exit-services IPR vs NR
14. GUESS "client pool ..." -> /developers/rust/client-pool (+tour: client creation takes several seconds)
15. GUESS "client storage persistent identity keys" -> tour#persist-your-identity, addressing#privacy-considerations
16. GUESS "which ports allowed exit policy" -> ONLY operator iptables dumps; authoritative list is an off-docs URL. FAILURE.
17. GUESS "Rust SDK socks5 module" -> /developers/rust/socks5 (+ link to /developers/concepts/exit-security)
18. LINK -> /developers/concepts/exit-security (pin an exit, which exit each package uses)
19. network_summary -> 850 nodes, 60 mixnodes, 80 entry / 100 exit gateways
20. GUESS "DNS leak smolmix" -> mix-dns (browser); native DNS story only in smolmix examples table
21. GUESS "gRPC HTTP/2 over mixnet" -> nothing direct; only deprecated TcpProxy ordering note. FAILURE.
22. GUESS "reliability retries SURBs" -> anonymous-replies, acks
23. GUESS "desktop always-on bandwidth" -> limitations#it-costs-the-same-when-idle; /developers#choosing-a-package (matrix)
24. GUESS "blockchain RPC wallet guide" -> demos/ens (browser-only), demos/railgun, /developers/chain (Nyx, irrelevant)
25. validate_sdk_config probe -> TS SetupMixTunnelOpts only, useless for native Rust
26. GUESS "Nym vs Tor" -> threat-model/comparisons
27. GUESS "end-to-end config" -> threat-model/configurations/end-to-end
