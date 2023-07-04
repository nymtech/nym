import { createNodeTesterClient, NodeTester } from '@nymproject/sdk';

let nodeTester: NodeTester | null = null;

/**
 * Display messages that have been sent up the websocket. Colours them blue.
 *
 * @param {string} message
 */
function displayOutput(message: string, color?: string) {
  const timestamp = new Date().toISOString().substr(11, 12);

  const sendDiv = document.createElement('div');
  const paragraph = document.createElement('p');
  paragraph.setAttribute('style', `color: ${color || 'blue'}`);
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
  const nodeTesterId = new Date().toISOString(); // make a new tester id for each session
  await nodeTester.tester.init(nymApiUrl, nodeTesterId);

  const mixnodes = await (await fetch(`${nymApiUrl}/v1/mixnodes/active`)).json();

  const exampleMixnodeIdentityKey = mixnodes[0].bond_information.mix_node.identity_key;

  const testButton: HTMLButtonElement = document.querySelector('#test-button') as HTMLButtonElement;
  const reconnectButton: HTMLButtonElement = document.querySelector('#reconnect-button') as HTMLButtonElement;
  const disconnectButton: HTMLButtonElement = document.querySelector('#disconnect-button') as HTMLButtonElement;
  const terminateButton: HTMLButtonElement = document.querySelector('#terminate-button') as HTMLButtonElement;

  const mixnodeIdInput = document.getElementById('mixnodeId') as HTMLFormElement;

  mixnodeIdInput.value = exampleMixnodeIdentityKey;

  reconnectButton.onclick = async function () {
    try {
      await nodeTester?.tester.reconnectToGateway();
    } catch (e: any) {
      console.error('Error', e);
      displayOutput(`ERROR: ${e.message}`, 'red');
    }
  };

  disconnectButton.onclick = async function () {
    try {
      await nodeTester?.tester.disconnectFromGateway();
    } catch (e: any) {
      console.error('Error', e);
      displayOutput(`ERROR: ${e.message}`, 'red');
    }
  };

  terminateButton.onclick = async function () {
    try {
      await nodeTester?.terminate();
    } catch (e: any) {
      console.error('Error', e);
      displayOutput(`ERROR: ${e.message}`, 'red');
    }
  };

  if (testButton) {
    testButton.onclick = async function () {
      console.log('clicked');

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
        try {
          const response = await nodeTester.tester.startTest(mixnodeId);
          displayOutput('Done!');
          if (response) {
            displayOutput(JSON.stringify(response, null, 2), 'green');
          }
        } catch (e: any) {
          console.error('Error', e);
          displayOutput(`ERROR: ${e.message}`, 'red');
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
