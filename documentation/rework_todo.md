Initial overhaul
----------------
- [x] init nextra project
- [x] work out dir structure
- [x] ~~QU pull ts sdk docs in here, or make nextra docs in rust sdk dir?~~ Pull tssdk docs in here

```
/documentation
├── autodocs/ <- rename
├── new_nextra_docs/ <- rename
├── README.md
├── post_process.sh (if we still need this)
└── scripts/
    └── <other_scripts_if_needed_still>
```

- [x] make sub_dirs for components, code snippets, etc as well (e.g. ts sdk specific ones, docs specific ones if necessary, etc)
- [x] start moving stuff over and check existing works
  - [ ] docs
    - [x] finish pass @ all pages, at least minimal stubs
    - [x] one-pager
    - [x] split chain stuff into integrations (interacting with the chain) and then move rest into the architecture subsection?
    - [ ] make sure to pull ecash docs in
    - [x] ~~diagrams~~ (moved to new features)
    - [ ] remove TODO links

  - [ ] operators
    - [ ] tabs / general condensation of page length
    - [ ] unify steps convention
    - [ ] variable component / page
    - [ ] community page with resources (explorers, forum, etc)
    - [ ] prerequisites
      - [ ] own page
      - [ ] admonishment at the top of other pages pointing @ prerequisites / RTFM

  - [ ] pull in all docs updates after 37d9f919227cec148e0355404b4029c0949e4dea from develop !!
   - [ ] operators docs (various)
   - [ ] ProxyClient/Server docs: formulate new integrations and docs around this

  - [ ] developers
    - [ ] overhaul the FAQs and split out into integrations section
    - [ ] integrations: pull all integration stuff from the TS SDK into its own subdir
    - [ ] rewrite around the idea of using the proxy logic for the moment, in order for ease of integration
    - [ ] write ovrview for the proxy
    - [x] rust sdk - move to its own dir
    - [x] proper code imports - static for the moment, look @ automation in next steps
    - [x] redo cargo file example
    - [ ] @ top: intro / quickstart / keyconcepts pages
  - [ ] ts sdk docs
    - [ ] quick content overhaul
      - [ ] remove whitelist references: replace with tornull
      - [ ] general
    - [ ] remove numbering from sidebar
  - [ ] check all links work and fix

- [ ] replace `mdbook-cmdrun`
- [ ] all images to root images dir with subdirs
- [ ] change mdbook admonishes to nextra admonish

- [ ] make common links (e.g. github) variables if we require several
- [ ] remove dangling index

- [ ] redo all diagrams - look @ tool Mark linked in Zulip
  - [ ] existing
  - [ ] packets
  - [ ] trace a packet

Autodocs
--------
- [x] autodocs branch - merge in
- [ ] work out where `autodocs` sits in CI
- [ ] `autodocs` pages - incorporate

CI / deployment
---------------
- [ ] try and get rid of as many random scripts from github CI as possible
- [ ] redirects on server
- [ ] QU shall we move to our own server?
- [ ] add redirect page + sitemap for old links
- [ ] new readme

For Frontend
------------
  - [ ] sidebar: collapsed by default?
  - [ ] landing page
  - [x] level selection (sdks, network, developers, operators) in top right
  - [x] make SDKs a dropdown menu to choose between
  - [ ] search in sidebar
  - [ ] ~~toc @ top of each page, dropdown (default collapsed)~~
  - [ ] new colours
  - [ ] links in footer
  - [x] get rid of discord
  - [ ] link to matrix

New Features
------------
- [ ] QU how to automatically pull in the rust sdk examples?
- [ ] QU what do we do with the ts sdk examples & how to automate?
- [ ] total docs rework
  - [x] key concepts
  - [ ] crypto systems
  - [x] architecture
- [ ] interactive wasm code a la lowlvl.academy
  - [ ] follow the packet
    - [ ] message
    - [ ] ip packet
  - [ ] docs content overhaul
    - [x] cryptosystems used
    - [ ] transport protocols used
    - [ ] poisson process
    - [ ] arch: why nym !p2p
  - [ ] anatomy of sphinx packet
- [ ] rust playground for clients + examples
- [ ] network/arch/clients: breakdown of a client path from startup (startup, gateway connection, what keys what auth format etc, auth, message sending, poisson process)
- [ ] network/concepts/mixing: diagram
- [ ] network/crypto/sphinx: diagrams
- [ ] network/concepts/surbs: diagram
- [ ] network/traffic: diagram
- [ ] network/traffic/acks: diagram
