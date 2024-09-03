# Nym deb meta packages

## Nymtech repo setup

`nym-repo-setup.deb` is a debian package that sets up the nymtech debian repo by copying the keyring file and adding `nymtech.list` to `/etc/apt/sources.list.d`.

## Nym VPN meta package

A basic meta package which only purpose is to depend on the daemon and UI.

# Build

They can all be built by running `make`.
