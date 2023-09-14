# sdk-version-bump

simple tool to update version numbers of our sdk packages.

## Usage:

note: this tool is expected to be run during CI, but if one wants to do it manually:

### For releases:

1. run `./sdk-version-bump remove-suffix` that will remove the `-rc.X` suffixes from all relevant packages
2. build everything and publish it to npm
3. run `./sdk-version-bump bump-version` that will update the versions of all relevant packages from `X.Y.Z` into `X.Y.(Z+1)-rc.0`. It will also update the `@nymproject/...` dependencies from `">=X.Y.Z-rc.0 || ^X"` to `">=X.Y.(Z+1)-rc.0 || ^X"`

### For pre-releases:

1. run `./sdk-version-bump bump-version --pre-release` that will update the release candidate version of all relevant packages from `X.Y.Z-rc.W` to `X.Y.Z-rc.(W+1)`

To run it from the root, do: `cargo run -p sdk-version-bump`