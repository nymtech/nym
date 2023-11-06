const fs = require('fs');

const packageName = '@nymproject/mix-fetch-wasm';
const packageJsonPath = require.resolve(packageName + '/package.json');

const packageJson = JSON.parse(fs.readFileSync(packageJsonPath).toString());

console.log(`🟢🟢🟢 ${packageName} is at ${packageJsonPath} is version ${packageJson.version}`);
