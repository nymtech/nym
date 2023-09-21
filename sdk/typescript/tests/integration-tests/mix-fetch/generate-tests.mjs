/* eslint-disable no-await-in-loop,no-restricted-syntax */
import fs from 'fs';
import path from 'path';
import child_process from 'child_process';
import http from 'http';
import handler from 'serve-handler';
import { runTests } from './tests/tests.mjs';

const GENERATED_DIR = 'generated';
const OUTPUT_DIR = `${GENERATED_DIR}/packages`;
const BASE_DIST_PATH = '../../../../../dist/ts/sdk/mix-fetch';
const packages = ['cjs', 'cjs-full-fat', 'esm', 'esm-full-fat'];

const doBuild = true;
const logTests = true;

// disabling deleting to preserve node_modules
// fs.rmSync(OUTPUT_DIR, { recursive: true, force: true });

fs.mkdirSync(OUTPUT_DIR, { recursive: true });

async function serve(publicDir) {
  const server = http.createServer((request, response) =>
    // see https://github.com/vercel/serve-handler#options for more options
    handler(request, response, { public: publicDir }),
  );

  server.listen(3000, () => {
    console.log(`  🌈 Running at http://localhost:3000, serving ${publicDir}...`);
  });

  return server;
}
function changeFileLocation(packageJsonPath, dependencyPath, kind) {
  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath).toString());
  packageJson.name = `${packageJson.name}-${kind}`;
  delete packageJson.dependencies['@nymproject/mix-fetch'];
  packageJson.dependencies[`@nymproject/mix-fetch${kind}`] = `file:${dependencyPath}`;
  fs.writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2));
}

console.log('🚚 Copying workspace...');
fs.cpSync('template/workspace/package.json', path.resolve(`${GENERATED_DIR}/package.json`));

packages.forEach((p) => {
  const dir = `${OUTPUT_DIR}/${p}`;
  if (doBuild) {
    fs.rmSync(dir, { recursive: true, force: true });
  }
  console.log(`🚚 Creating ${dir}...`);

  let kind = p.startsWith('cjs') ? '-commonjs' : '';
  if (p.endsWith('-full-fat')) {
    kind += '-full-fat';
  }

  const importScript = '<script type="module" src="./index.ts"></script>';
  const importStatement = `import { mixFetch } from '@nymproject/mix-fetch${kind}';`;
  const pluginPackageJsonName = `@nymproject/mix-fetch${kind}`;

  const src = fs.readFileSync('template/src/index.ts').toString().replace("'$IMPORT_STATEMENT';", importStatement);
  const html = fs.readFileSync('template/src/index.html').toString().replace('<!-- $IMPORT -->', importScript);
  const plugins = fs
    .readFileSync('template/webpack/webpack.plugins.js')
    .toString()
    .replaceAll('$PACKAGE', pluginPackageJsonName);

  const pluginsEmpty = 'module.exports = { plugins: [] };';

  fs.cpSync('template/parcel', `${dir}/parcel`, { recursive: true });
  fs.cpSync('template/src', `${dir}/parcel/src`, { recursive: true });

  fs.cpSync('template/webpack', `${dir}/webpack`, { recursive: true });
  fs.cpSync('template/src', `${dir}/webpack/src`, { recursive: true });

  fs.writeFileSync(`${dir}/webpack/src/index.ts`, src);
  fs.writeFileSync(`${dir}/parcel/src/index.ts`, src);
  // fs.writeFileSync(`${dir}/webpack/src/index.html`, html);
  fs.writeFileSync(`${dir}/parcel/src/index.html`, html);
  fs.writeFileSync(`${dir}/webpack/webpack.plugins.js`, p.endsWith('-full-fat') ? pluginsEmpty : plugins);

  changeFileLocation(`${dir}/parcel/package.json`, path.resolve(`${BASE_DIST_PATH}/${p}`), kind);
  changeFileLocation(`${dir}/webpack/package.json`, path.resolve(`${BASE_DIST_PATH}/${p}`), kind);
});

console.log('✅ Generated\n');

console.log(`🚀 Installing workspace packages in ${path.resolve(OUTPUT_DIR)}...`);
const resultNpmInstall = child_process.spawnSync('npm', ['install'], { cwd: path.resolve(OUTPUT_DIR) });
if (resultNpmInstall.status !== 0) {
  console.log(resultNpmInstall.stdout.toString());
  console.log(resultNpmInstall.stderr.toString());
  console.log('❌ Failed to install dependencies');
  process.exit(-1);
}

if (doBuild) {
  packages.forEach((p) => {
    const dir = `${OUTPUT_DIR}/${p}`;

    ['parcel', 'webpack'].forEach((kind) => {
      const project = `${dir}/${kind}`;
      console.log(`🚀 Building ${path.resolve(project)}...`);

      const result = child_process.spawnSync('npm', ['run', 'build'], { cwd: path.resolve(project) });

      if (result.status !== 0) {
        console.log(`❌ Failed to build ${project}`);
        console.log(result.stdout.toString());
        console.log(result.stderr.toString());
        process.exit(-1);
      } else {
        console.log(
          child_process.spawnSync('ls', ['-lah'], { cwd: `${path.resolve(project)}/dist` }).stdout.toString(),
        );
      }
    });
  });
}

console.log('✅ Built\n');

console.log('👀 Testing...');

const summary = [];
let totalTimeMilliseconds = 0;
let failures = 0;
for (const p of packages) {
  const dir = `${OUTPUT_DIR}/${p}`;

  const kinds = [
    'parcel', // simple parcel project
    'webpack', // webpack, sometimes with the CopyPlugin to add the WASM bundles
  ];
  for (const kind of kinds) {
    const project = `${dir}/${kind}`;
    console.log(`🔎 Testing ${path.resolve(project)}...`);

    const server = await serve(path.resolve(project, 'dist'));

    const start = performance.now();
    let success = false;
    let errors = [];
    try {
      console.log('🚀 About to run tests...');
      errors = await runTests(logTests);
      if (errors.length > 0) {
        errors.forEach((e) => console.log(e.text));
      } else {
        success = true;
      }
    } catch (e) {
      failures += 1;
      errors.push({ text: e.message, type: 'error' });
    } finally {
      server.closeAllConnections();
      server.close();
    }
    const end = performance.now();
    const duration = Math.floor(end - start);
    totalTimeMilliseconds += duration;

    if (errors.length) {
      console.log(`❌ Tests failed for ${project} with ${errors.length} errors:`);
      console.table(errors);
    } else {
      console.log('✅ OK');
    }

    summary.push({
      variant: p,
      bundler: kind,
      duration: `${duration}ms`,
      success: success ? '✅ OK' : `❌ Failed with ${errors.length} errors`,
      errors: errors.map((e) => e.text).join('\n'),
    });

    console.log();
  }
}

console.log();
console.log('Summary:');
console.table(summary);

console.log();
console.log(`Tests took ${Math.floor(totalTimeMilliseconds / 1000)} seconds to run`);
console.log(`${failures > 0 ? `❌ Done with ${failures} errors` : '✅ Done'}`);
