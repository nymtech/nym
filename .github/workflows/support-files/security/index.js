const Handlebars = require('handlebars');
const fs = require('fs');
const path = require('path');
const { Octokit, App } = require('octokit');

async function addToContextAndValidate(context) {
    return
}

async function getMessageBody(context) {
  const source = fs
    .readFileSync("deny.message").toString();

  return source;
}

module.exports = {
  addToContextAndValidate,
  getMessageBody,
};
