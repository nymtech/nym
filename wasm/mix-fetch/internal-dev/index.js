// Copyright 2020-2023 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

class WebWorkerClient {
  worker = null;

  constructor() {
    this.worker = new Worker("./worker.js");

    this.worker.onmessage = (ev) => {
      if (ev.data && ev.data.kind) {
        switch (ev.data.kind) {
          case "DisplayString":
            const { rawString } = ev.data.args;
            displayReceivedRawString(rawString);
            break;
        }
      }
    };
  }

  doFetch = (target) => {
    if (!this.worker) {
      console.error("Could not send message because worker does not exist");
      return;
    }

    this.worker.postMessage({
      kind: "FetchPayload",
      args: {
        target,
      },
    });
  };
}

let client = null;

async function main() {
  client = new WebWorkerClient();

  const fetchButtonText = document.querySelector("#fetch-button");
  fetchButtonText.onclick = function (e) {
    document.getElementById("output").innerHTML = "";
    if ($("#fetch_payload").val().trim() === "") {
      e.preventDefault();
      let errorDiv = document.createElement("div");
      let paragraph = document.createElement("p");
      paragraph.style.color = "red";
      paragraph.innerText = "please enter a valid request!!";

      errorDiv.appendChild(paragraph);
      document.getElementById("output").appendChild(errorDiv);
      return false;
    }

    doFetch();
  };
}

async function doFetch() {
  const url = document.getElementById("fetch_payload").value;
  const returnJson = document.getElementById("returnJsonToggle").checked;

  displaySend(`Fetching: ${url}`);

  try {
    const response = await fetch(url);

    if (!response.ok) {
      throw new Error(`HTTP error! Status: ${response.status}`);
    }

    let data;
    if (returnJson) {
      data = await response.json();
    } else {
      data = await response.text();
    }

    displayReceivedRawString(data);
  } catch (error) {
    console.error("There was a problem fetching the data:", error);
  }
}

/**
 * Display messages that have been sent up the websocket. Colours them blue.
 *
 * @param {string} message
 */
function displaySend(message) {
  let timestamp = new Date().toISOString().substr(11, 12);

  let sendDiv = document.createElement("div");
  let paragraph = document.createElement("p");
  paragraph.style.color = "blue";
  paragraph.innerText = timestamp + " sent >>> " + message;

  sendDiv.appendChild(paragraph);
  document.getElementById("output").appendChild(sendDiv);
}

function displayReceivedRawString(raw) {
  let timestamp = new Date().toISOString().substr(11, 12);
  let receivedDiv = document.createElement("div");
  receivedDiv.style.overflow = "auto";
  receivedDiv.style.wordWrap = "break-word";

  let paragraph = document.createElement("p");
  paragraph.style.color = "green";
  paragraph.style.fontWeight = "bold";
  paragraph.innerText = timestamp + " received >>> " + JSON.stringify(raw);

  receivedDiv.appendChild(paragraph);

  document.getElementById("output").appendChild(receivedDiv);

  let lineBreak = document.createElement("br");
  document.getElementById("output").appendChild(lineBreak);
}

main();
