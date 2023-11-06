import fs from 'fs';

const packageJson = JSON.parse(fs.readFileSync('package.json').toString());

const devWorkspace = ['sdk/typescript/packages/**', 'sdk/typescript/examples/**', 'sdk/typescript/codegen/**'];
if (!packageJson.workspaces.includes(devWorkspace)) {
  // add
  packageJson.workspaces.push(...devWorkspace);

  // write out modified file
  fs.writeFileSync('package.json', JSON.stringify(packageJson, null, 2));
}
