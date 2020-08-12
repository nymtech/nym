var ourAddress;
var connection;

// Please note that this javascript is extremely bad, it's only purpose is to show some basic API calls, not how
// a proper application should have been written.

async function main() {
    var port = '1977' // client websocket listens on 1977 by default, change if yours is different
    var localClientUrl = "ws://127.0.0.1:" + port;

    // Set up and handle websocket connection to our desktop client.
    connection = await connectWebsocket(localClientUrl).then(function (c) {
        return c;
    }).catch(function (err) {
        display("Websocket connection error. Is the client running with <pre>--connection-type WebSocket</pre> on port " + port + "?");
    })
    connection.onmessage = function (e) {
        handleResponse(e);
    };

    sendSelfAddressRequest();

    // Set up the send button
    const sendButton = document.querySelector('#send-button');
    sendButton.onclick = function () {
        sendMessageToMixnet();
    }
}

// Handle any messages that come back down the websocket. 
function handleResponse(resp) {
    // hacky workaround for receiving pushed 'text' messages,
    // basically we can either receive proper server responses, i.e. 'error', 'send', 'selfAddress'
    // or actual messages, without any framing, so they do not have 'type' field
    try {
        let response = JSON.parse(resp.data);
        if (response.type == "error") {
            displayJsonResponseWithoutReply("Server responded with error: " + response.message);
        } else if (response.type == "selfAddress") {
            displayJsonResponseWithoutReply(response);
            ourAddress = response.address;
            display("Our address is:  " + ourAddress + ", we will now send messages to ourself.");
        } else if (response.type == "received") {
            handleReceivedTextMessage(response)
        }
    } catch (_) {
        displayJsonResponseWithoutReply(resp.data)
    }
}


function handleReceivedTextMessage(message) {
    console.log("received a message!")
    const text = message.message
    const replySurb = message.replySurb

    if (replySurb != null) {
        displayJsonResponseWithReply(text, replySurb)
    } else {
        displayJsonResponseWithoutReply(text)
    }
}

// Send a message to the mixnet. 
function sendMessageToMixnet() {
    const sendText = document.getElementById("sendtext").value;
    const surbCheckbox = document.querySelector('#with-surb');
    const attachReplySURB = surbCheckbox.checked;

    const message = {
        type: "send",
        message: sendText,
        recipient: ourAddress,
        withReplySurb: attachReplySURB,
    }

    displayJsonResponseWithoutReply(message);
    connection.send(JSON.stringify(message));
}

function sendReplyMessageToMixnet(messageContent, replySurb) {
    const message = {
        type: "reply",
        message: messageContent,
        replySurb: replySurb,
    }

    displayJsonResponseWithoutReply(message);
    connection.send(JSON.stringify(message));
}

// Send a message to the mixnet client, asking what our own address is. 
// In this simplistic demo, we'll just use our own address to send ourselves messages. 
//
// In a real application, you might want to ensure that somebody else got your
// address so that they could send messages to you. 
function sendSelfAddressRequest() {
    var selfAddress = {
        type: "selfAddress"
    }
    displayJsonSend(selfAddress);
    connection.send(JSON.stringify(selfAddress));
}

function display(message) {
    document.getElementById("output").innerHTML += "<p>" + message + "</p >";
}

function displayJsonSend(message) {
    let sendDiv = document.createElement("div")
    let paragraph = document.createElement("p")
    paragraph.setAttribute('style', 'color: blue')
    let paragraphContent = document.createTextNode("sent >>> " + JSON.stringify(message))
    paragraph.appendChild(paragraphContent)

    sendDiv.appendChild(paragraph)
    document.getElementById("output").appendChild(sendDiv)
}

function displayJsonResponseWithoutReply(message) {
    let receivedDiv = document.createElement("div")
    let paragraph = document.createElement("p")
    paragraph.setAttribute('style', 'color: green')
    let paragraphContent = document.createTextNode("received >>> " + JSON.stringify(message) + "(NO REPLY AVAILABLE)")
    paragraph.appendChild(paragraphContent)

    receivedDiv.appendChild(paragraph)
    document.getElementById("output").appendChild(receivedDiv)
}

function displayJsonResponseWithReply(message, replySurb) {
    let replyBox = document.createElement("input")
    replyBox.setAttribute('type', 'text');
    replyBox.setAttribute('value', 'type your anonymous reply here!');
    replyBox.setAttribute('size', 50);

    let sendButton = document.createElement("button")
    let buttonText = document.createTextNode("Send")
    sendButton.appendChild(buttonText)

    sendButton.onclick = () => {
        sendReplyMessageToMixnet(replyBox.value, replySurb)
    }

    let receivedDiv = document.createElement("div")
    let paragraph = document.createElement("p")
    paragraph.setAttribute('style', 'color: green')
    let paragraphContent = document.createTextNode("received >>> " + JSON.stringify(message) + "(HERE BE SURB)")
    paragraph.appendChild(paragraphContent)

    receivedDiv.appendChild(paragraph)
    receivedDiv.appendChild(replyBox)
    receivedDiv.appendChild(sendButton)

    document.getElementById("output").appendChild(receivedDiv)
}

// Connect to a websocket. 
function connectWebsocket(url) {
    return new Promise(function (resolve, reject) {
        var server = new WebSocket(url);
        server.onopen = function () {
            resolve(server);
        };
        server.onerror = function (err) {
            reject(err);
        };

    });
}

// Start it!
main();
