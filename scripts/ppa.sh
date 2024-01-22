#/bin/bash
# Run from repository root
# Takes PPA_SIGNING_KEY base64 encoded as first argument

echo $1 | base64 -d >ppa-signing-key.asc
gpg --import ppa-signing-key.asc
rm ppa-signing-key.asc

cargo deb -p nym-mixnode
cargo deb -p nym-gateway
cargo deb -p nym-cli

mv target/debian/*.deb ppa/debian

cd ppa

dpkg-scanpackages --multiversion . >Packages
gzip -k -f Packages

apt-ftparchive release . >Release
gpg --default-key "nym@nymtech.net" -abs -o - Release >Release.gpg
gpg --default-key "nym@nymtech.net" --clearsign -o - Release >InRelease
