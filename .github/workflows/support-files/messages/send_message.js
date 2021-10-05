const Bot = require('keybase-bot');
const Handlebars = require('handlebars');
const fs = require('fs');

async function main() {
  const data = { env: process.env };
  // const data = { ...PASTE TEST DATA HERE ... }; // -- DEV: uncomment to set test data

  // validation of environment
  if(!(process.env.NYM_PROJECT_NAME || data.env.NYM_PROJECT_NAME)) {
    throw new Error('Please set env var NYM_PROJECT_NAME with the project name for displaying in notification messages');
  }

  // extract the git branch name
  const GIT_BRANCH_NAME = (process.env.GITHUB_REF || data.env.GITHUB_REF).split('/').slice(2).join('/');

  data.env.GIT_BRANCH_NAME = GIT_BRANCH_NAME;
  const source = fs
    .readFileSync(process.env.IS_SUCCESS === 'true' ? 'success' : 'failure')
    .toString();
  const template = Handlebars.compile(source);
  const result = template(data);

  // -- DEV: uncomment to show what is available in the handlebars template / show the result
  // console.dir({ data }, { depth: null });
  // console.log(result);

  const bot = new Bot();
  try {
    const username = process.env.KEYBASE_NYMBOT_USERNAME;
    const paperkey = process.env.KEYBASE_NYMBOT_PAPERKEY;

    if(!username) {
      throw new Error('Username is not defined. Please set env var KEYBASE_NYMBOT_USERNAME');
    }
    if(!paperkey) {
      throw new Error('Paperkey is not defined. Please set env var KEYBASE_NYMBOT_PAPERKEY');
    }

    console.log(`Initialising keybase with user "${username}" and key: "${'*'.repeat(paperkey.length)}"...`);
    await bot.init(username, paperkey, { verbose: false });

    const channel = {
      name: 'nymtech_bot',
      membersType: 'team',
      topicName: 'testing',
      topic_type: 'CHAT',
    };
    const message = {
      body: result,
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

main();
