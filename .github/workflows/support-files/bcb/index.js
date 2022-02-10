const Handlebars = require('handlebars');
const fs = require('fs');
const path = require('path');

async function addToContextAndValidate(context) {
  if (!context.env.NYM_CI_WWW_LOCATION) {
    throw new Error('Please ensure the env var NYM_CI_WWW_LOCATION is set');
  }
  if (!context.env.NYM_CI_WWW_BASE) {
    throw new Error('Please ensure the env var NYM_CI_WWW_BASE is set');
  }
}

async function getMessageBody(context) {
  const source = fs
    .readFileSync(
      context.env.IS_SUCCESS === 'true'
        ? path.resolve(__dirname, 'templates', 'success')
        : path.resolve(__dirname, 'templates', 'failure'),
    )
    .toString();
  const template = Handlebars.compile(source);
  return template(context);
}

module.exports = {
  addToContextAndValidate,
  getMessageBody,
};
