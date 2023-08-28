const Handlebars = require('handlebars');
const fs = require('fs');
const path = require('path');
const { Octokit, App } = require('octokit');

async function addToContextAndValidate(context) {
  return
}

async function getMessageBody(context) {
  try {
    const source = fs
    .readFileSync("./notifications/deny.message").toString();
    return source;
} catch (error) {
    console.error(error);
}

}

module.exports = {
  addToContextAndValidate,
  getMessageBody,
};
