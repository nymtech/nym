const fs = require("fs");
const path = require("path");

const directory = ".";

function convertJsonToJs(filePath) {
  const content = fs.readFileSync(filePath, "utf8");
  const jsonContent = JSON.parse(content);
  const jsContent = `module.exports = ${JSON.stringify(jsonContent, null, 2)};`;

  const newFilePath = filePath.replace(".json.bak", ".js");
  fs.writeFileSync(newFilePath, jsContent, "utf8");
  fs.unlinkSync(filePath); // Remove the original .json file
}

function processDirectory(dir) {
  const files = fs.readdirSync(dir);

  files.forEach((file) => {
    const fullPath = path.join(dir, file);
    const stat = fs.statSync(fullPath);

    if (stat.isDirectory()) {
      processDirectory(fullPath);
    } else if (file === "_meta.json.bak") {
      convertJsonToJs(fullPath);
    }
  });
}

processDirectory(directory);
