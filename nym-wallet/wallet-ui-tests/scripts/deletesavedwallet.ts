const { exec } = require("child_process")

const deleteSavedFile = exec("rm '/home/benedetta/.local/share/nym-wallet/saved-wallet.json'", (err, stdout, stderr) => {
    if (err) {
        console.error(`${err.message}`)
        return
    } else
        console.log("File deleted")
})
