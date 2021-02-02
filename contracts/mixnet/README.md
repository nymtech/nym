# CosmWasm Starter Pack

This is a template to build smart contracts in Rust to run inside a
[Cosmos SDK](https://github.com/cosmos/cosmos-sdk) module on all chains that enable it.
To understand the framework better, please read the overview in the
[cosmwasm repo](https://github.com/CosmWasm/cosmwasm/blob/master/README.md),
and dig into the [cosmwasm docs](https://www.cosmwasm.com).
This assumes you understand the theory and just want to get coding.

## Creating a new repo from template

Assuming you have a recent version of rust and cargo (v1.47.0+) installed
(via [rustup](https://rustup.rs/)),
then the following should get you a new repo to start a contract:

First, install
[cargo-generate](https://github.com/ashleygwilliams/cargo-generate).
Unless you did that before, run this line now:

```sh
cargo install cargo-generate --features vendored-openssl
```

Now, use it to create your new contract.
Go to the folder in which you want to place it and run:

**0.13 (latest)**

```sh
cargo generate --git https://github.com/CosmWasm/cosmwasm-template.git --name PROJECT_NAME
````

**0.12**

```sh
cargo generate --git https://github.com/CosmWasm/cosmwasm-template.git --branch 0.12 --name PROJECT_NAME
```

**0.11**

```sh
cargo generate --git https://github.com/CosmWasm/cosmwasm-template.git --branch 0.11 --name PROJECT_NAME
```
**0.10**

```sh
cargo generate --git https://github.com/CosmWasm/cosmwasm-template.git --branch 0.10 --name PROJECT_NAME
```

**0.9**

```sh
cargo generate --git https://github.com/CosmWasm/cosmwasm-template.git --branch 0.9 --name PROJECT_NAME
```

**0.8**

```sh
cargo generate --git https://github.com/CosmWasm/cosmwasm-template.git --branch 0.8 --name PROJECT_NAME
```

You will now have a new folder called `PROJECT_NAME` (I hope you changed that to something else)
containing a simple working contract and build system that you can customize.

## Create a Repo

After generating, you have a initialized local git repo, but no commits, and no remote.
Go to a server (eg. github) and create a new upstream repo (called `YOUR-GIT-URL` below).
Then run the following:

```sh
# this is needed to create a valid Cargo.lock file (see below)
cargo check
git checkout -b master # in case you generate from non-master
git add .
git commit -m 'Initial Commit'
git remote add origin YOUR-GIT-URL
git push -u origin master
```

## CI Support

We have template configurations for both [GitHub Actions](.github/workflows/Basic.yml)
and [Circle CI](.circleci/config.yml) in the generated project, so you can
get up and running with CI right away.

One note is that the CI runs all `cargo` commands
with `--locked` to ensure it uses the exact same versions as you have locally. This also means
you must have an up-to-date `Cargo.lock` file, which is not auto-generated.
The first time you set up the project (or after adding any dep), you should ensure the
`Cargo.lock` file is updated, so the CI will test properly. This can be done simply by
running `cargo check` or `cargo unit-test`.

## Using your project

Once you have your custom repo, you should check out [Developing](./Developing.md) to explain
more on how to run tests and develop code. Or go through the
[online tutorial](https://www.cosmwasm.com/docs/getting-started/intro) to get a better feel
of how to develop.

[Publishing](./Publishing.md) contains useful information on how to publish your contract
to the world, once you are ready to deploy it on a running blockchain. And
[Importing](./Importing.md) contains information about pulling in other contracts or crates
that have been published.

Please replace this README file with information about your specific project. You can keep
the `Developing.md` and `Publishing.md` files as useful referenced, but please set some
proper description in the README.
