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

  constructor(onComplete) {
    this.worker = new Worker("./worker.js");

    this.worker.onmessage = (ev) => {
      if (ev.data && ev.data.kind) {
        switch (ev.data.kind) {
          case "DisplayString":
            const { rawString } = ev.data.args;
            displayReceivedRawString(rawString);
            break;
        }
        if(onComplete) {
          onComplete();
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
  client = new WebWorkerClient(() => {
    fetchButton.disabled = false;
  });

  const fetchButton = document.querySelector("#fetch-button");

  fetchButton.onclick = function (e) {
    if (fetchButton.disabled) {
      alert("Processing... Please wait!");
      return;
    }

    document.getElementById("output").innerHTML = "";

    if ($("#fetch_payload").val().trim() === "") {
      e.preventDefault();
      let errorDiv = document.createElement("div");
      let paragraph = document.createElement("p");
      paragraph.style.color = "red";
      paragraph.innerText = "Please enter a valid request!!";

      errorDiv.appendChild(paragraph);
      document.getElementById("output").appendChild(errorDiv);
      return false;
    }

    fetchButton.disabled = true;

    client.doFetch($("#fetch_payload").val().trim());
  };
}

async function doFetch() {
  document
    .getElementById("fetch-button")
    .addEventListener("click", async () => {
      const url = document.getElementById("fetch_payload").value;
      //introduce toggle in the future
      //const returnJson = document.getElementById("returnJsonToggle").checked;

      try {
        await client.doFetch(url);
      } catch (error) {
        console.error("There was a problem fetching the data:", error);
      }
    });
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
