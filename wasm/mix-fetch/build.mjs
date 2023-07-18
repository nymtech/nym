import fs from 'fs';

const packageJson = JSON.parse(fs.readFileSync('../../dist/wasm/mix-fetch/package.json').toString());

if (!packageJson.files.includes('go_conn.wasm')) {
    packageJson.files.push('go_conn.wasm');
}
if (!packageJson.files.includes('wasm_exec.js')) {
    packageJson.files.push('wasm_exec.js');
}

fs.writeFileSync('../../dist/wasm/mix-fetch/package.json', JSON.stringify(packageJson, null, 2));