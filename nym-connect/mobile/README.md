<!--
Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
SPDX-License-Identifier: Apache-2.0
-->

# Nym Connect - Mobile

[Nym Connect](https://github.com/nymtech/nym/tree/develop/nym-connect) application for Mobile.

Nym Connects sets up a SOCKS5 proxy for local applications to use.

**NOTE**: Currently we only focus on Android,
the remaining docs apply for Android development only.

## Installation prerequisites - Linux / Mac

- `Yarn`
- `NodeJS >= v16`
- `Rust & cargo`
- Android development environment (JDK, SDK/NDK, AVD etc...)

For setting up an Android development environment see
https://next--tauri.netlify.app/next/guides/getting-started/prerequisites/linux#android

## Installation

Inside the `mobile/nym-connect` directory, run the following commands:

```
yarn install
yarn prewebpack:dev
```

## Development

Assuming there is a running android [emulator](https://developer.android.com/studio/run/emulator)
or a real [device](https://developer.android.com/studio/run/device) connected.
Inside the `mobile/nym-connect/src-tauri` directory, run the following command:

```
yarn dev:android
```

#### Debugging

https://next--tauri.netlify.app/next/guides/debugging/application#mobile

## Production

To build the APK, run the build commands.

```
yarn webpack:prod
WRY_ANDROID_PACKAGE=net.nymtech.nym_connect WRY_ANDROID_LIBRARY=nym_connect cargo tauri android build --debug --apk
```

**NOTE**: Production build without the `--debug` flag requires a signed build.

# Storybook

Run storybook with:

```
yarn storybook
```

And build storybook static site with:

```
yarn storybook:build
```
