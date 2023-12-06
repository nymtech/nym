# Documentation

## Doc projects
Each directory contains a readme with more information about running and contributing to the projects. Each is built with [`mdbook`](https://rust-lang.github.io/mdBook/index.html) - use `mdbook serve` to build and serve them (defaults to `localhost:3000`).
* `docs` contains technical documentation hosted at [https://nymtech.net/docs](https://nymtech.net/docs)
* `dev-portal` contains developer documentation hosted at [https://nymtech.net/developers](https://nymtech.net/developers)
* `operators` contains node setup and maintenance guides hosted at [https://nymtech.net/operators](https://nymtech.net/operators)

> If you are looking for the Typescript SDK documentation located at [sdk.nymtech.net](https://sdk.nymtech.net) this can be found in `../sdk/typescript/docs/`

## Contribution 
* If you wish to add to the documentation please create a PR against this repo. 
* If you are **adding a plugin dependency** make sure to also **add that to the list of plugins in `install_mdbook_deps.sh` line 12**. 

## Scripts 
* `bump_versions.sh` allows you to update the ~~`platform_release_version` and~~ `wallet_release_version` variable~~s~~ in the `book.toml` of each mdbook project at once. You can also optionally update the `minimum_rust_version` as well. Helpful for lazy-updating when cutting a new version of the docs. 

* The following scripts are used by the `ci-dev.yml` and `cd-dev.yml` scripts (located in `../.github/workflows/`):
  * `build_all_to_dist.sh` is used for building all mdbook projects and moving the rendered html to `../dist/` to be rsynced with various servers. 
  * `install_mdbook_deps.sh` checks for an existing install of mdbook (and plugins), uninstalls them, and then installs them on a clean slate. This is to avoid weird dependency clashes if relying on an existing mdbook version. 
  * `post_process.sh` is used to post process CSS/image/href links for serving several mdbooks from a subdirectory. 
  * `removed_existing_config.sh` is used to check for existing nym client/node config files on the CI/CD server and remove it if it exists. This is to mitigate issues with `mdbook-cmdrun` where e.g. a node is already initialised, and the command fails.   

## CI/CD 
Deployment of the docs is partially automated and partially manual. 
* `ci-docs.yml` will run on pushes to all branches **apart from `master`**
* `cd-docs.yml` must be run manually. This pushes to a staging branch which then must be manually promoted to production. 

## Licensing and copyright information
This is a monorepo and components that make up Nym as a system are licensed individually, so for accurate information, please check individual files.

As a general approach, licensing is as follows this pattern:

* <p xmlns:cc="http://creativecommons.org/ns#" xmlns:dct="http://purl.org/dc/terms/"><a property="dct:title" rel="cc:attributionURL" href="https://nymtech.net/docs">Nym Documentation</a> by <a rel="cc:attributionURL dct:creator" property="cc:attributionName" href="https://nymtech.net">Nym Technologies</a> is licensed under <a href="http://creativecommons.org/licenses/by-nc-sa/4.0/?ref=chooser-v1" target="_blank" rel="license noopener noreferrer" style="display:inline-block;">CC BY-NC-SA 4.0<img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/cc.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/by.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/nc.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/sa.svg?ref=chooser-v1"></a></p>

* Nym applications and binaries are [GPL-3.0-only](https://www.gnu.org/licenses/)

* Used libraries and different components are [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0.html) or [MIT](https://mit-license.org/)

