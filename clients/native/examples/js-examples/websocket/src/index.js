var ourAddress;

async function main() {
    var port = '1977' // client websocket listens on 1977 by default, change if yours is different
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

    sendSelfAddressRequest(connection);

    // Set up the send button
    const sendButton = document.querySelector('#send-button');
    sendButton.onclick = function () {
        sendMessageToMixnet(connection);
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
            displayJsonResponse("Server responded with error: " + response.message);
        } else if (response.type == "selfAddress") {
            displayJsonResponse(response);
            ourAddress = response.address;
            display("Our address is:  " + ourAddress + ", we will now send messages to ourself.");
        }
    } catch (_) {
        displayJsonResponse(resp.data)
    }

}

// Send a message to the mixnet. 
function sendMessageToMixnet(connection) {
    var sendText = document.getElementById("sendtext").value;
    var message = {
        type: "send",
        message: sendText,
        recipient: ourAddress
    }

    displayJsonSend(message);
    connection.send(JSON.stringify(message));
}

// Send a message to the mixnet client, asking what our own address is. 
// In this simplistic demo, we'll just use our own address to send ourselves messages. 
//
// In a real application, you might want to ensure that somebody else got your
// address so that they could send messages to you. 
function sendSelfAddressRequest(connection) {
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
