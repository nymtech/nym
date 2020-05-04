var ourAddress;

async function main() {
    var port = '9001' // client websocket listens on 9001 by default, change if yours is different
    var localClientUrl = "ws://127.0.0.1:" + port;

    // Set up and handle websocket connection to our desktop client.
    var connection = await connectWebsocket(localClientUrl).then(function (c) {
        return c;
    }).catch(function (err) {
        display("Websocket connection error. Is the client running with <pre>--connection-type WebSocket</pre> on port " + port + "?");
    })
    connection.onmessage = function (e) {
        handleResponse(e);
    };


    sendOwnDetailsRequest(connection);
    pollForMessages(connection);

    // Set up the send button
    const sendButton = document.querySelector('#send-button');
    sendButton.onclick = function () {
        sendMessageToMixnet(connection);
    }
}

// Handle any messages that come back down the websocket. 
function handleResponse(resp) {
    let response = JSON.parse(resp.data);
    if (response.type == "error") {
        displayJsonResponse("Server responded with error: " + response);
    } else if (response.type == "fetch") {
        if (response.messages.length > 0) {
            displayJsonResponse(response);
        }
    } else if (response.type == "ownDetails") {
        displayJsonResponse(response);
        ourAddress = response.address;
        display("Our address is:  " + ourAddress + ", we will now send messages to ourself.");
    }
}

// Send a message to the mixnet. 
function sendMessageToMixnet(connection) {
    var sendText = document.getElementById("sendtext").value;
    var message = {
        type: "send",
        message: sendText,
        recipient_address: ourAddress
    }

    displayJsonSend(message);
    connection.send(JSON.stringify(message));
}

// Send a message to the mixnet client, asking what our own address is. 
// In this simplistic demo, we'll just use our own address to send ourselves messages. 
//
// In a real application, you might want to ensure that somebody else got your
// address so that they could send messages to you. 
function sendOwnDetailsRequest(connection) {
    var ownDetails = {
        type: "ownDetails"
    }
    displayJsonSend(ownDetails);
    connection.send(JSON.stringify(ownDetails));
}

// Periodically poll for any messages waiting for us on the mixnet. This is
// an annoying source of latency and will soon go away. 
function pollForMessages(connection) {
    setInterval(() => {
        var message = {
            type: "fetch"
        }
        connection.send(JSON.stringify(message));
    }, 1000);
}

function display(message) {
    document.getElementById("output").innerHTML += "<p>" + message + "</p >";
}

function displayJsonSend(message) {
    document.getElementById("output").innerHTML += "<p style='color: blue;'>sent >>> " + JSON.stringify(message) + "</p >";
}

function displayJsonResponse(message) {
    document.getElementById("output").innerHTML += "<p style='color: green;'>received <<<" + JSON.stringify(message) + "</p >";
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
