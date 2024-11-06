# Nym Docs v2

This is v2 of the nym docs, condensed from various mdbooks projects that we had previously.

These docs are hosted at [nymtech.net/docs](www.nymtech.net/docs).

## Doc projects
`docs/pages/` contains several subdirs, each hosting a subsection of the docs:
* `network` contains key concepts, cryptosystems, architecture.
* `developers` contains key concepts for developers, required architecture, and Rust/Typescript SDK docs.
* `operators` contains node setup and maintenance guides.

## Local development
### Dependencies
Our `prebuild` script relies on the following:
- `python`
- `pip`
- [`pandas`](https://pandas.pydata.org/)
- [`tabulate`](https://pypi.org/project/tabulate/)
- `jq`

Otherwise make sure to have `node` installed.

### Serve Local (Hot Reload)
```sh
pnpm i
pnpm run dev
```

Open `http://localhost:3000`.

## Build
```sh
pnpm run build
```

## Contribution
* If you wish to add to the documentation please create a PR against this repo, with a `patch` against `develop`.

## Scripts
* `generate:commands`: generates command output files for clients and binaries. This script runs the `autodoc` rust binary, moves the files to their required places, and then if there is an update, commits them to git. We commit the files as our remote deployments pull from a git repo. **Only run this script on branches where you want to push e.g. the build info of a binary to production docs**; it will build the monorepo binaries and use their command output for the produced markdown files.
* `generate:tables`: generates various information tables containing some repo-wide variables and information about ISPs.

### Autodoc
`autodoc` is a script that generates markdown files containing commands and their output (both command and `--help` output). For the moment the binaries and their commands are manually configured in the script.

> **Only run this script on branches where you want to push e.g. the build info of a binary to production docs**; it will build the monorepo binaries and use their command output for the produced markdown files.

## Licensing and copyright information
This is a monorepo and components that make up Nym as a system are licensed individually, so for accurate information, please check individual files.

As a general approach, licensing is as follows this pattern:

* <p xmlns:cc="http://creativecommons.org/ns#" xmlns:dct="http://purl.org/dc/terms/"><a property="dct:title" rel="cc:attributionURL" href="https://nymtech.net/docs">Nym Documentation</a> by <a rel="cc:attributionURL dct:creator" property="cc:attributionName" href="https://nymtech.net">Nym Technologies</a> is licensed under <a href="http://creativecommons.org/licenses/by-nc-sa/4.0/?ref=chooser-v1" target="_blank" rel="license noopener noreferrer" style="display:inline-block;">CC BY-NC-SA 4.0<img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/cc.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/by.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/nc.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/sa.svg?ref=chooser-v1"></a></p>

* Nym applications and binaries are [GPL-3.0-only](https://www.gnu.org/licenses/)

* Used libraries and different components are [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0.html) or [MIT](https://mit-license.org/)
