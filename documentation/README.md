# Documentation

## Doc projects
Each directory contains a readme with more information about running and contributing to the projects. Each is built with [`mdbook`](https://rust-lang.github.io/mdBook/index.html) - use `mdbook serve` to build and serve them (defaults to `localhost:3000`).
* `docs` contains technical documentation hosted at [https://nymtech.net/docs](https://nymtech.net/docs)
* `dev-portal` contains developer documentation hosted at [https://nymtech.net/developers](https://nymtech.net/developers)
* `operators` contains node setup and maintenance guides hosted at [https://nymtech.net/operators](https://nymtech.net/operators)

## Scripts 
* `bump_versions.sh` allows you to update the ~~`platform_release_version` and~~ `wallet_release_version` variable~~s~~ in the `book.toml` of each mdbook project at once. You can also optionally update the `minimum_rust_version` as well. Helpful for lazy-updating when cutting a new version of the docs. 
* `build_all_to_dist.sh` is used by the `ci-dev.yml` and `cd-dev.yml` scripts for building all mdbook projects and moving the rendered html to `../dist/` to be rsynced with various servers. 
* `post_process.sh` is a script called by the github CI and CD workflows to post process CSS/image/href links for serving several mdbooks from a subdirectory. 


### Licensing and copyright information
This is a monorepo and components that make up Nym as a system are licensed individually, so for accurate information, please check individual files.

# Licensing

As a general approach, licensing is as follows this pattern:

* <p xmlns:cc="http://creativecommons.org/ns#" xmlns:dct="http://purl.org/dc/terms/"><a property="dct:title" rel="cc:attributionURL" href="https://nymtech.net/docs">Nym Documentation</a> by <a rel="cc:attributionURL dct:creator" property="cc:attributionName" href="https://nymtech.net">Nym Technologies</a> is licensed under <a href="http://creativecommons.org/licenses/by-nc-sa/4.0/?ref=chooser-v1" target="_blank" rel="license noopener noreferrer" style="display:inline-block;">CC BY-NC-SA 4.0<img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/cc.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/by.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/nc.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/sa.svg?ref=chooser-v1"></a></p>

* Nym applications and binaries are [GPL-3.0-only](https://www.gnu.org/licenses/)

* Used libraries and different components are [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0.html) or [MIT](https://mit-license.org/)

Again, for accurate information, please check individual files.
