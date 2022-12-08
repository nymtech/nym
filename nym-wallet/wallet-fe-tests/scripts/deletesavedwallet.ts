const { exec } = require('child_process');
const os = require('os');

let homedir = os.homedir();

const doesFileExist = exec(`test -f ${homedir}/.local/share/nym-wallet/saved-wallet.json`, (err, stdout, stderr) => {
  if (err) {
    console.error(`${err.message}`);
    return;
  } else console.log('File deleted');
});
