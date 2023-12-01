import fs from 'fs';

const packageJson = JSON.parse(fs.readFileSync('package.json').toString());

const devWorkspace = ['sdk/typescript/packages/**', 'sdk/typescript/examples/**', 'sdk/typescript/codegen/**'];

// remove
packageJson.workspaces = packageJson.workspaces.filter((w) => !devWorkspace.includes(w));

// write out modified file
fs.writeFileSync('package.json', JSON.stringify(packageJson, null, 2));
