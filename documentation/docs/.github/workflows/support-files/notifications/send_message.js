require('dotenv').config();

const { sendMatrixMessage } = require('./send_message_to_matrix');

let context = {
  kinds: ['ci-docs','cd-docs'],
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
  if (context.env.MATRIX_ROOM) {
    if (!context.env.MATRIX_SERVER) {
      throw new Error(
        'Matrix server is not defined. Please set env var MATRIX_SERVER',
      );
    }
    if (!context.env.MATRIX_USER_ID) {
      throw new Error(
        'Matrix user id is not defined. Please set env var MATRIX_USER_ID',
      );
    }
    if (!context.env.MATRIX_TOKEN) {
      throw new Error(
        'Matrix token is not defined. Please set env var MATRIX_TOKEN',
      );
    }
    if (!context.env.MATRIX_DEVICE_ID) {
      throw new Error(
        'Matrix device id is not defined. Please set env var MATRIX_DEVICE_ID',
      );
    }
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

  if (!context.env.GIT_BRANCH_NAME) {
    context.env.GIT_BRANCH_NAME = context.env.GITHUB_REF.split('/')
      .slice(2)
      .join('/');
  }

  context.status = process.env.IS_SUCCESS === 'true' ? 'success' : 'failure';
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
  if(context.env.MATRIX_ROOM) {
    await sendMatrixMessage(context, messageBody, context.env.MATRIX_ROOM)
  }
}

// call main function and let NodeJS handle the promise
main();
