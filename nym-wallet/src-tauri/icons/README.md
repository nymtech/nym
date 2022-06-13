# Regenerating icons

Do the following to regenerate the icons:

```
cd ~
git clone nym ...
cd nym
cd nym-wallet
npx @tauri-apps/tauricon ../assets/appicon/appicon.png
cp src-tauri/icons/128x128.png src-tauri/icons/32x32.png
```
