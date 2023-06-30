import { createNodeTesterClient, NodeTester } from '@nymproject/sdk';

let nodeTester: NodeTester | null = null;

/**
 * Display messages that have been sent up the websocket. Colours them blue.
 *
 * @param {string} message
 */
function displayOutput(message: string) {
  const timestamp = new Date().toISOString().substr(11, 12);

  const sendDiv = document.createElement('div');
  const paragraph = document.createElement('p');
  paragraph.setAttribute('style', 'color: blue');
  const paragraphContent = document.createTextNode(`${timestamp} >>> ${message}`);
  paragraph.appendChild(paragraphContent);

  sendDiv.appendChild(paragraph);
  document.getElementById('output')?.appendChild(sendDiv);
}

/**
 * The main entry point
 */
async function main() {
  nodeTester = await createNodeTesterClient();

  // add node tester to the Window globally, so that it can be used from the dev tools console
  (window as any).nodeTester = nodeTester;

  if (!nodeTester) {
    console.error('Oh no! Could not the node test');
    return;
  }

  const nymApiUrl = 'https://validator.nymtech.net/api';

  const sendButton: HTMLButtonElement = document.querySelector('#send-button') as HTMLButtonElement;

  const mixnodeIdInput = document.getElementById('mixnodeId') as HTMLFormElement;

  if (sendButton) {
    sendButton.onclick = async function () {
      const mixnodeId = mixnodeIdInput.value;
      if (!nodeTester) {
        displayOutput('ERROR: The node tester is not defined');
        console.error('The node tester is not defined');
        return;
      }
      if (!mixnodeId) {
        displayOutput('ERROR: No mix id specified');
        console.error('No mix id specified');
        return;
      }

      if (nodeTester && mixnodeId) {
        displayOutput('Starting test...');
        const response = await nodeTester.tester.startTest(mixnodeId);
        displayOutput('Done!');
        if (response) {
          const { score, sentPackets, receivedPackets, receivedAcks, duplicatePackets, duplicateAcks } = response;
          displayOutput(
            JSON.stringify(
              {
                score,
                sentPackets,
                receivedPackets,
                receivedAcks,
                duplicatePackets,
                duplicateAcks,
              },
              null,
              2,
            ),
          );
        }
      }
    };
  }
}

// wait for the html to load
window.addEventListener('DOMContentLoaded', () => {
  // let's do this!
  main();
});
