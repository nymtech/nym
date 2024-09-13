# Nym Developer Portal
Developer Portal for the Nym privacy platform built using the [mdBook](https://rust-lang.github.io/mdBook/) docs framework. Deployed version can be found [here](https://nymtech.net/developers).

## Contributing
Contributions to our documentation are very welcome. Please work on your contribution in either a `feature/<feature-name>` or `chore/<chore-name>` branch from `master` and target your pull request at `master`.

Changes merged to `master` will be autodeployed to the production site.

### Adding community projects and resources
If you have built a project with Nym or are compiling and writing resources about Nym, we want to include your work in the `community-resources/` section to share with the rest of the community! Just follow the existing formatting and add your project to the page, then put in a pull request.

## Variables
There are some variables that are shared across this book, such as the current latest software version.

Variables are denoted in the `.md` files wrapped in `{{}}` (e.g `{{wallet_release_version}}`), and are located in the `book.toml` file under the `[preprocessor.variables.variables]` heading. If you are changing something like the software release version, minimum code versions in prerequisites, etc, **check in here first!**

## Building
When working locally, it is recommended that you use `mdbook serve` to have a local version of the docs served on `localhost:3000`, with hot reloading on any changes made to files in the `src/` directory.

You can find other commands in the [mdBook CLI tool docs](https://rust-lang.github.io/mdBook/cli/index.html).

### I tried to edit files in `theme/` and they aren't taking effect / `mdbook serve` causes a looping reload on file changes after changing fields in `[preprocessor.theme]` config

Looping reload is a known issue with the `mdbook-theme` preprocessor used for the table of contents and layout of these docs. As outlined in the `mdbook-theme` [readme](https://github.com/zjp-CN/mdbook-theme#avoid-repeating-call-on-this-tool-when-mdbook-watch) one way to mitigate this is to set `turn-off = true` under `[preprocessor.theme]`. This means that `mdbook serve` or `mdbook watch` ignores changes to the `theme/` directory, which is the source of the looping reload. If you have changed or commented out this line, reintroduce it to remove the looping reload. If you are trying to edit the theme of the docs and want to apply the change, see [here](https://github.com/zjp-CN/mdbook-theme#avoid-repeating-call-on-this-tool-when-mdbook-watch) for more info on how to remove the block, change the theme, and reintroduce the block.
