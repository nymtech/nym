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

```
/new_docs/pages
├── sdks
|   ├── rust/
|   └── typescript/
├── operators/
├── docs/
└── developers
```

- [x] make sub_dirs for components, code snippets, etc as well (e.g. ts sdk specific ones, docs specific ones if necessary, etc)
- [ ] start moving stuff over and check existing works
  - [ ] docs
  - [ ] operators
  - [ ] developers
    - [x] rust sdk - move to its own dir
    - [ ] proper code imports - static for the moment, look @ automation in next steps
    - [ ] redo cargo file example
    - [ ] @ top: intro / quickstart / keyconcepts pages
  - [ ] ts sdk docs
    - [ ] quick content overhaul
      - [ ] remove whitelist references: replace with tornull
      - [ ] general
    - [ ] remove numbering from sidebar
  - [ ] check all links work and fix
- [ ] autodocs branch - merge in
- [ ] make common links (e.g. github) variables if we require several

- [ ] new theme
  - [ ] landing page
  - [x] level selection (sdks, network, developers, operators) in top right
  - [ ] make SDKs a dropdown menu to choose between
  - [ ] search in sidebar
  - [ ] toc @ top of each page, dropdown (default collapsed)
  - [ ] new colours
  - [ ] links in footer
  - [x] get rid of discord
  - [ ] link to matrix

- [ ] pull integration decision tree out of ts sdk and into dev portal - make its own subsection
- [ ] add landing page for ease of navigation
- [ ] try and get rid of as many random scripts from github CI as possible
- [ ] redirects on server
- [ ] QU shall we move to our own server?
- [ ] redo all diagrams
- [ ] add redirect page + sitemap for old links
- [ ] new readme

New Features
------------
- [ ] make sure to pull ecash docs in
- [ ] QU how to automatically pull in the rust sdk examples?
- [ ] QU what do we do with the ts sdk examples & how to automate?
- [ ] total docs rework
  - [ ] key concepts
  - [ ] crypto systems
  - [ ] architecture
- [ ] interactive wasm code a la lowlvl.academy
  - [ ] follow the packet
    - [ ] message
    - [ ] ip packet
  - [ ] docs content overhaul
    - [ ] cryptosystems used
    - [ ] transport protocols used
    - [ ] poisson process
    - [ ] arch: why nym !p2p
  - [ ] anatomy of sphinx packet
- [ ] rust playground for clients + examples
