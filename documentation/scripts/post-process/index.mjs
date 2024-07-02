import { unified } from "unified";
import parse from "rehype-parse";
import inspectUrls from "@jsdevtools/rehype-url-inspector";
import stringify from "rehype-stringify";
import { read, write } from "to-vfile";
import path from "path";
import fs from "fs";
import { glob } from "glob";

async function main() {
  const distDir = "../../../dist/docs";

  const items = [];

  const books = [
    'developers',
    'docs',
    'operators',
  ];

  for (const book of books) {
    // only process the root `index.html` files, because they have absolute paths instead of relative paths
    const filenames = [path.resolve(distDir, book, 'index.html')];

    // leaving this here for a future where other files need to be processed
    // const filenames = await glob(path.resolve(distDir, book) + '/**/*.html');

    for (const f of filenames) {
      // Create a Rehype processor with the inspectUrls plugin
      const processor = unified()
        .use(parse)
        .use(inspectUrls, {
          inspectEach(args) {
            const { url: rawUrl, propertyName } = args;
            const { tagName } = args.node;
            const filename = args.file.history[0];

            const relativeFilename = path.relative(distDir, filename);
            const relativeDirectory = path.dirname(relativeFilename);

            // remove relative paths from URL
            const bareUrl = rawUrl.split('/').filter(c => c !== '.' && c !== '..').join('/');
            let url;

            if (rawUrl.includes('.html#')) {
              url = path.join(`/${relativeDirectory}`, bareUrl);
            } else if (rawUrl.startsWith('#')) {
              url = path.join(`/`, relativeFilename + bareUrl);
            } else {
              url = path.join(`/${book}`, bareUrl);
            }

            // const item = { filename, relativeDirectory, tagName, propertyName, rawUrl, url };
            const item = { tagName, rawUrl, url };

            if (tagName === 'a') {
              console.log(args);
            }

            if (!rawUrl.startsWith('http')) {
              if (tagName === 'link' || tagName === 'script' || tagName === 'a') {
                args.node.properties[propertyName] = url;
                items.push(item);
              }
            }
          }
        })
        .use(stringify);

      // Read the example HTML file
      const filename = path.resolve(distDir, f);
      console.log(`${filename}...`);

      let file = await read(filename);

      // Crawl the HTML file and find all the URLs
      const res = await processor.process(file);

      fs.writeFileSync(filename, res.value);
    }
  }

  console.table(items);
  console.log();
}

main();
