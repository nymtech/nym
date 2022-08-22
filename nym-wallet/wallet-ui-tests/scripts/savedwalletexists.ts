const { exec } = require("child_process")

// const doesFileExist = exec("test -f /home/benedetta/.local/share/nym-wallet/saved-wallet.json" && "echo '$FILE exists.'" || "echo 'file doesn't exist'")
// scriptExist ? expect(getErrorWarning).toStrictEqual(textConstants.invalidPasswordOnSignIn) : expect(getErrorWarning).toStrictEqual(textConstants.failedToFindWalletFile)


const doesFileExist = exec("test -f /home/benedetta/.local/share/nym-wallet/saved-wallet.json", (err, stdout, stderr) => {
        if (err) {
            console.error(`${err.message}`)
            return
        } else
            console.log("File: " + stdout)
    })
    