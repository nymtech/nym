# Nym mobile client

Work in Progress

Nym client port to mobile devices

## Dependencies

### Install rust targets

    # Android targets
    rustup target add aarch64-linux-android i686-linux-android x86_64-linux-android

    # iOS targets
    rustup target add aarch64-apple-ios x86_64-apple-ios 

### Rust dependencies 

    # this cargo subcommand will help you create a universal library for use with iOS.
    cargo install cargo-lipo
    # this tool will let you automatically create the C/C++11 headers of the library.
    cargo install cbindgen
    # to install android ndk support
    cargo install cargo-ndk

## Build    

### iOS

    $ cargo lipo --release

### Android 

    $ cargo ndk --target aarch64-linux-android --android-platform 22 -- build --release

### Export C headers from Rust code 

    $ cbindgen src/lib.rs -l c > nym_mobile.h

### Sample code repository ####

https://github.com/mileschet/nymmobile-flutter
