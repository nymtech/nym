const { exec } = require("child_process")

const doesFileExist = exec("test -f /home/benedetta/.local/share/nym-wallet/saved-wallet.json")
