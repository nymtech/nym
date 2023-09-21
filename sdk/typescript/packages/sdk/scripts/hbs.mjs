import Handlebars from "handlebars";
import fs from 'fs';
import path from 'path';
import glob from 'glob';

const filename = process.argv[2];
const outputFilename = process.argv[3];
console.log(`Processing file ${filename} as ${outputFilename}...`);
const template = Handlebars.compile(fs.readFileSync(filename).toString());

glob.sync('./partials/*.md').forEach(g => {
    const key = path.parse(g).name;
    console.log(`Registering partial ${g} as ${key}...`);
    Handlebars.registerPartial(key, fs.readFileSync(g).toString());
});

console.log('Done');
console.log();

const output = template({});

console.log(fs.writeFileSync(outputFilename, output));

