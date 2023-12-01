import * as fs from 'fs';

// parse the package.json from the SDK, so we can keep fields like the name and version
const json = JSON.parse(fs.readFileSync('package.json').toString());

// defaults (NB: these are in the output file locations)
const browser = 'index.js';
const main = 'index.js';
const types = 'index.d.ts';

const getPackageJson = (type, suffix) => ({
  name: `${json.name}${suffix ? `-${suffix}` : ''}`,
  version: json.version,
  license: json.license,
  author: json.author,
  type,
  browser,
  main,
  types,
});

fs.writeFileSync('dist/cjs/package.json', JSON.stringify(getPackageJson('commonjs', 'commonjs'), null, 2));
