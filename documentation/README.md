# Documentation

## Doc projects
Each directory contains a readme with more information about running and contributing to the projects. Each is built with [`mdbook`](https://rust-lang.github.io/mdBook/index.html) - use `mdbook serve` to build and serve them (defaults to `localhost:3000`).
* `docs` contains technical documentation hosted at [https://nymtech.net/docs](https://nymtech.net/docs)
* `dev-portal` contains developer documentation hosted at [https://nymtech.net/developers](https://nymtech.net/developers)
* `operators` contains node setup and maintenance guides hosted at [https://nymtech.net/operators](https://nymtech.net/operators)

> If you are looking for the Typescript SDK documentation located at [sdk.nymtech.net](https://sdk.nymtech.net) this can be found in `../sdk/typescript/docs/`

## Scripts 
* `bump_versions.sh` allows you to update the ~~`platform_release_version` and~~ `wallet_release_version` variable~~s~~ in the `book.toml` of each mdbook project at once. You can also optionally update the `minimum_rust_version` as well. Helpful for lazy-updating when cutting a new version of the docs. 

* The following scripts are used by the `ci-dev.yml` and `cd-dev.yml` scripts:
  * `build_all_to_dist.sh` is used for building all mdbook projects and moving the rendered html to `../dist/` to be rsynced with various servers. 
  * `install_mdbook_deps.sh` checks for an existing install of mdbook (and plugins), uninstalls them, and then installs them on a clean slate. This is to avoid weird dependency clashes if relying on an existing mdbook version. 
  * `post_process.sh` is used to post process CSS/image/href links for serving several mdbooks from a subdirectory. 

[//]: # (  * `removed_existing_config.sh` is used to remove existing nym client/node config files on the CI/CD server so avoid `mdbook-cmdrun` command errors.  )

### Licensing and copyright information
This is a monorepo and components that make up Nym as a system are licensed individually, so for accurate information, please check individual files.

As a general approach, licensing is as follows this pattern:
- applications and binaries are GPLv3
- libraries and components are Apache 2.0 or MIT
- documentation is Apache 2.0 or CC0-1.0

For accurate information, please check individual files.
