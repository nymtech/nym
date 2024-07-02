import fs from 'fs';
import yaml from 'js-yaml';

// Load the pnpm-workspace.yaml file
const pnpmWorkspaceYamlPath = 'pnpm-workspace.yaml';
const pnpmWorkspaceContent = fs.readFileSync(pnpmWorkspaceYamlPath, 'utf8');
const pnpmWorkspace = yaml.load(pnpmWorkspaceContent);

// Define the workspaces to add
const devWorkspace = ['sdk/typescript/packages/**', 'sdk/typescript/examples/**', 'sdk/typescript/codegen/**'];

// Check if the workspaces are already included, if not, add them
if (!pnpmWorkspace.packages) {
  pnpmWorkspace.packages = [];
}
const missingWorkspaces = devWorkspace.filter((ws) => !pnpmWorkspace.packages.includes(ws));
if (missingWorkspaces.length > 0) {
  pnpmWorkspace.packages.push(...missingWorkspaces);

  // Write out the modified pnpm-workspace.yaml file
  const newYamlContent = yaml.dump(pnpmWorkspace);
  fs.writeFileSync(pnpmWorkspaceYamlPath, newYamlContent, 'utf8');
}
