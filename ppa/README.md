# Nymtech repo setup

`nym-repo-setup.deb` is a debian package that sets up the nymtech debian repo by copying the keyring file and adding `nymtech.list` to `/etc/apt/sources.list.d`.

## Building the debian package

To build the debian file run

```sh
$ dpkg-deb --build nym-repo-setup
```

