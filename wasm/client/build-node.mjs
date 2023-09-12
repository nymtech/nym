import fs from 'fs';

const packageJson = JSON.parse(fs.readFileSync('../../dist/node/wasm/client/package.json').toString());

packageJson.name = `${packageJson.name}-node`;

fs.writeFileSync('../../dist/node/wasm/client/package.json', JSON.stringify(packageJson, null, 2));
