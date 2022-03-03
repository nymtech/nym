require('dotenv').config();

const Bot = require('keybase-bot');

let context = {
  kinds: ['network-explorer', 'nightly'],
};

/**
 * Validate that all required env and context vars are available
 */
function validateContext() {
  if (!context.env.NYM_NOTIFICATION_KIND) {
    throw new Error(
      'Please set env var NYM_NOTIFICATION_KIND with the project kind that matches a directory in ".github/workflows/support-files"',
    );
  }
  if (!context.kinds.includes(context.env.NYM_NOTIFICATION_KIND)) {
    throw new Error(`Env var NYM_NOTIFICATION_KIND is not in ${context.kinds}`);
  }
  if (!context.env.NYM_PROJECT_NAME) {
    throw new Error(
      'Please set env var NYM_PROJECT_NAME with the project name for displaying in notification messages',
    );
  }
  if (!context.env.KEYBASE_NYM_CHANNEL) {
    throw new Error(
      'Please set env var KEYBASE_NYM_CHANNEL with the channel name for the notification message',
    );
  }
  if (!context.env.KEYBASE_NYMBOT_USERNAME) {
    throw new Error(
      'Username is not defined. Please set env var KEYBASE_NYMBOT_USERNAME',
    );
  }
  if (!context.env.KEYBASE_NYMBOT_PAPERKEY) {
    throw new Error(
      'Paperkey is not defined. Please set env var KEYBASE_NYMBOT_PAPERKEY',
    );
  }
}

/**
 * Creates a context that will be available in the templates for rendering notifications
 */
function createTemplateContext() {
  const options = { dateStyle: 'full', timeStyle: 'long' };
  context.timestamp = new Date().toLocaleString(undefined, options);

  // add environment to template context and validate
  context.env = process.env;
  try {
    validateContext();
  } catch (e) {
    if(process.env.SHOW_DEBUG) {
      // recursively print the context for easy debugging and rethrow the error
      console.dir({ context }, { depth: null });
    }
    throw e;
  }

  context.kind = context.env.NYM_NOTIFICATION_KIND;

  context.keybase = {
    channel: context.env.KEYBASE_NYM_CHANNEL,
    username: context.env.KEYBASE_NYMBOT_USERNAME,
    paperkey: context.env.KEYBASE_NYMBOT_PAPERKEY,
  };

  if (!context.env.GIT_BRANCH_NAME) {
    context.env.GIT_BRANCH_NAME = context.env.GITHUB_REF.split('/')
      .slice(2)
      .join('/');
  }

  context.status = process.env.IS_SUCCESS === 'true' ? 'success' : 'failure';
}

async function sendKeybaseMessage(messageBody) {
  const bot = new Bot();
  try {
    console.log(
      `Initialising keybase with user "${
        context.keybase.username
      }" and key: "${'*'.repeat(context.keybase.paperkey.length)}"...`,
    );
    await bot.init(context.keybase.username, context.keybase.paperkey, {
      verbose: false,
    });

    const channel = {
      name: 'nymtech_bot',
      membersType: 'team',
      topicName: context.keybase.channel,
      topic_type: 'CHAT',
    };
    const message = {
      body: messageBody,
    };

    console.log(`Sending to ${channel.name}#${channel.topicName}...`);
    await bot.chat.send(channel, message);

    console.log('Message sent!');
  } catch (error) {
    console.error(error);
    process.exitCode = -1;
  } finally {
    await bot.deinit();
  }
}

/**
 * Uses the `kind` set in the context to process the context and generate a notification message
 * @returns {Promise<string>} A string notification message body
 */
async function processKindScript() {
  const script = require(`../${context.kind}`);
  if (!script.addToContextAndValidate) {
    throw new Error(
      `"./${context.kind}/index.js" does not export a method called "async addToContextAndValidate(context)"`,
    );
  }
  if (!script.getMessageBody) {
    throw new Error(
      `"./${context.kind}/index.js" does not export a method called "async getMessageBody(context)"`,
    );
  }

  // call the script to modify and validate the context
  await script.addToContextAndValidate(context);

  // let the script create a message body and return the result as a string for sending
  return await script.getMessageBody(context);
}

/**
 * The main function, as async so that await syntax is available
 */
async function main() {
  createTemplateContext();
  console.log(`Sending notification for kind "${context.kind}"...`);
  const messageBody = await processKindScript();
  if(process.env.SHOW_DEBUG) {
    console.log('-----------------------------------------');
    console.log(messageBody);
    console.log('-----------------------------------------');
  }
  await sendKeybaseMessage(messageBody);
}

// call main function and let NodeJS handle the promise
main();
