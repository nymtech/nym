# Regenerating icons

> **Note**: This is likely to be temporary until `tauri icon` is put back into the CLI.

The Tauri Docs say to use the CLI to generate icons: https://tauri.studio/docs/api/cli/#icon. However `1.0.0-rc.X` appears to not have this command. `1.0.0-beta.6` does 🎉!

Do the following to regenerate the icons:

```
cd ~
git clone nym ...
cd nym
docker run -v "$(pwd)":/workspace -it node:16 /bin/bash
npm i -g @tauri-apps/cli@1.0.0-beta.6
cd /workspace/nym-wallet/src-tauri
tauri icon /workspace/assets/appicon/appicon.png
exit
```

Reasons to use docker:

- you can't destroy your dev environments `npm` cache
- if you mess it up, kill the container, try again
- inside the `src-tauri` directory, `node` will resolve to the nearest `node_modules` directory and you'll get the wrong `tauri` cli