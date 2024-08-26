Initial overhaul
----------------
- [x] init nextra project
- [ ] work out dir structure
- [x] ~~QU pull ts sdk docs in here, or make nextra docs in rust sdk dir?~~ Pull tssdk docs in here

```
/documentation
├── autodocs/
├── new_nextra_docs/
├── <other_scripts_if_needed_still>
├── README.md
├── post_process.sh (if we still need this)
└── scripts/
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

- [ ] make sub_dirs for components, code snippets, etc as well (e.g. ts sdk specific ones, docs specific ones if necessary, etc)
- [ ] start moving stuff over and check existing works
  - [ ] docs
  - [ ] operators
  - [ ] developers
  - [ ] ts sdk docs
    - [ ] remove numbering from sidebar
  - [ ] others?
- [ ] autodocs branch
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
- [ ] add redirect page + sitemap for old links
- [ ] new readme

New Features
------------
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
