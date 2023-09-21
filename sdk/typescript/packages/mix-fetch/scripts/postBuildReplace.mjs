import fs from 'fs';
import path from 'path';

// first level is a list of file names to process, then for each copying the @rollup/replace plugin syntax here:
//  key is what to search for, value is what to replace it with
const replaceConfig = {
  'esm/web-worker-0.js': {
    // add `import.meta.url` to make WASM file relative to the worker JS file
    "_loadWasmModule(0, 'mix_fetch_wasm_bg.wasm',":
      "_loadWasmModule(0, new URL('mix_fetch_wasm_bg.wasm', import.meta.url),",
    "_loadWasmModule(0, 'go_conn.wasm',": "_loadWasmModule(0, new URL('go_conn.wasm', import.meta.url),",
  },
  'esm/index.js': {
    // tell the browser the worker is a module, so it should provide `import.meta.url` needed above
    'const worker = new WorkerFactory();': "const worker = new WorkerFactory({ type: 'module' });",
  },
};

const basePathToFindFilesIn = process.argv[2];

console.log(`Replacing files in "${path.resolve(basePathToFindFilesIn)}"...`);

Object.keys(replaceConfig).forEach((filename) => {
  const absFilename = path.resolve(basePathToFindFilesIn, filename);

  if (!fs.existsSync(absFilename)) {
    console.log(`Skipping replacing ${filename} as does not exist`);
    return;
  }

  const content = fs.readFileSync(absFilename).toString();

  console.log(`Replacing values in "${absFilename}"...`);

  const replacementMap = replaceConfig[filename];

  let newContent = content;
  Object.keys(replacementMap).forEach((toFind) => {
    const toReplace = replacementMap[toFind];
    newContent = newContent.replaceAll(toFind, toReplace);
    fs.writeFileSync(absFilename, newContent);
  });

  console.log('Done');
});
