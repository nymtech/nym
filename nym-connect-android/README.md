<!--
Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
SPDX-License-Identifier: Apache-2.0
-->

# Nym Connect - Android

[Nym Connect](https://github.com/nymtech/nym/tree/develop/nym-connect) application for Android.

Nym Connects sets up a SOCKS5 proxy for local applications to use.

## Installation prerequisites - Linux / Mac

- `Yarn`
- `NodeJS >= v16`
- `Rust & cargo`
- Android development environment (JDK, SDK/NDK, AVD etc...)

For setting up an Android development environment see
https://next--tauri.netlify.app/next/guides/getting-started/prerequisites/linux#android

## Installation

Inside the `nym-connect-android` directory, run the following command:

```
yarn install
```

## Development

Assuming there is a running android [emulator](https://developer.android.com/studio/run/emulator)
or a real [device](https://developer.android.com/studio/run/device) connected.
Inside the `nym-connect-android/src-tauri` directory, run the following command:

```
WRY_ANDROID_PACKAGE=net.nymtech.nym_connect_android WRY_ANDROID_LIBRARY=nym_connect_android cargo tauri android dev
```

## Production

To build the application bundles (APK and AAB files), run the build command.

```
WRY_ANDROID_PACKAGE=net.nymtech.nym_connect_android WRY_ANDROID_LIBRARY=nym_connect_android cargo tauri android build
```

# Storybook

Run storybook with:

```
yarn storybook
```

And build storybook static site with:

```
yarn storybook:build
```
