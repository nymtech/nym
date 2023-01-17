import * as fs from 'fs';

// parse the package.json from the SDK, so we can keep fields like the name and version
const json = JSON.parse(fs.readFileSync('package.json').toString());

// defaults (NB: these are in the output file locations)
const browser = 'index.js';
const main = 'index.js';
const types = 'index.d.ts';

// make a package.json for the CommonJS bundle
const packageJsonCommonJS = {
  name: `${json.name}-commonjs`,
  version: json.version,
  license: json.license,
  author: json.author,
  type: 'commonjs',
  browser,
  main,
  types,
};

// make a package.json for the ESM bundle
const packageJsonESM = {
  name: json.name,
  version: json.version,
  license: json.license,
  author: json.author,
  type: 'module',
  browser,
  main,
  types,
};

fs.writeFileSync('dist/cjs/package.json', JSON.stringify(packageJsonCommonJS, null, 2));
fs.writeFileSync('dist/esm/package.json', JSON.stringify(packageJsonESM, null, 2));
