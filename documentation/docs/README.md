# Nym Documentation
Documentation for the Nym privacy platform built using the [mdBook](https://rust-lang.github.io/mdBook/) docs framework.

Documentation can be viewed at https://nymtech.net/docs

## Contributing
Contributions to our documentation are very welcome. Please work on your contribution in either a `feature/<feature-name>` or `chore/<chore-name>` branch from `master` and target your pull request at `master`.

Since these docs autogenerate command output and import docs from binaries in `target/release` on `build` make sure you're branching off of `master` when making your branch.

Changes merged to `master` will be autodeployed to the production site.

### Contributing a new translation
To contribute tranlsations in a new language, please get in touch via [Matrix](https://matrix.to/#/#general:nymtech.chat) or [Discord](https://discord.gg/FaTJb8q8).

### Variables
There are some variables that are shared across the entire docs site, such as the current latest software version.

Variables are denoted in the `.md` files wrapped in `{{}}` (e.g `{{wallet_release_version}}`), and are located in the `book.toml` file under the `[preprocessor.variables.variables]` heading. If you are changing something like the software release version, minimum code versions in prerequisites, etc, **check in here first!**

### Diagrams
Most diagrams are simply ascii. Copies are kept in `/diagrams/` for ease of reproducability. Created using [textik](https://textik.com/#).

### Importing files and auto-generated command output

Example files are inserted as per normal with mdbook.

Some binary command outputs are generated using the [`cmdrun`](https://docs.rs/mdbook-cmdrun/latest/mdbook_cmdrun/) mdbook plugin.

## Building
When working locally, it is recommended that you use `mdbook serve` to have a local version of the docs served on `localhost:3000`, with hot reloading on any changes made to files in the `src/` directory.

You can find other commands in the [mdBook CLI tool docs](https://rust-lang.github.io/mdBook/cli/index.html).

### I tried to edit files in `theme/` and they aren't taking effect / `mdbook serve` causes a looping reload on file changes after changing fields in `[preprocessor.theme]` config

Looping reload is a known issue with the `mdbook-theme` preprocessor used for the table of contents and layout of these docs. As outlined in the `mdbook-theme` [readme](https://github.com/zjp-CN/mdbook-theme#avoid-repeating-call-on-this-tool-when-mdbook-watch) one way to mitigate this is to set `turn-off = true` under `[preprocessor.theme]`. This means that `mdbook serve` or `mdbook watch` ignores changes to the `theme/` directory, which is the source of the looping reload. If you have changed or commented out this line, reintroduce it to remove the looping reload. If you are trying to edit the theme of the docs and want to apply the change, see [here](https://github.com/zjp-CN/mdbook-theme#avoid-repeating-call-on-this-tool-when-mdbook-watch) for more info on how to remove the block, change the theme, and reintroduce the block.

### Checking the mdBook version
To check the version of mdBook installed on your system, you can use the `mdbook --version` command. This will print the version number of mdBook installed on your system in the terminal.

The latest release of the binary of the pre-compiled binaries can be found on [GitHub](https://github.com/rust-lang/mdBook/releases).
