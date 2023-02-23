import * as fs from 'fs';

// parse the package.json from the SDK, so we can keep fields like the name and version
const json = JSON.parse(fs.readFileSync('./package.json').toString());

// defaults (NB: these are in the output file locations)
const main = 'index.js';
const types = 'index.d.ts';

// make a package.json for the bundle
const packageJson = {
  name: json.name,
  version: json.version,
  license: json.license,
  author: json.author,
  main,
  types,
};

fs.writeFileSync('./dist/package.json', JSON.stringify(packageJson, null, 2));
