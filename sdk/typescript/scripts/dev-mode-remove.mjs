import fs from 'fs';
import yaml from 'js-yaml';

// Load the pnpm-workspace.yaml file
const pnpmWorkspaceYamlPath = 'pnpm-workspace.yaml';
const pnpmWorkspaceContent = fs.readFileSync(pnpmWorkspaceYamlPath, 'utf8');
const pnpmWorkspace = yaml.load(pnpmWorkspaceContent);

const devWorkspace = ['sdk/typescript/packages/**', 'sdk/typescript/examples/**', 'sdk/typescript/codegen/**'];

// Remove specified workspaces
pnpmWorkspace.packages = pnpmWorkspace.packages.filter((w) => !devWorkspace.includes(w));

// Convert the modified object back to YAML
const newYamlContent = yaml.dump(pnpmWorkspace);

// Write out the modified pnpm-workspace.yaml file
fs.writeFileSync(pnpmWorkspaceYamlPath, newYamlContent, 'utf8');
