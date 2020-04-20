var ourAddress;

async function main() {
    var port = '9001' // client websocket listens on 9001 by default, change if yours is different
    var localClientUrl = "ws://127.0.0.1:" + port;

    var connection = await connectWebsocket(localClientUrl).then(function (c) {
        return c;
    }).catch(function (err) {
        display("Websocket ERROR: " + err);
    })

    connection.onmessage = function (e) {
        handleMessage(e);
    };

    sendOwnDetailsRequest(connection);

    pollForMessages(connection);

    const sendButton = document.querySelector('#send-button');
    sendButton.onclick = function () {
        sendMessageToMixnet(connection);
    }
}

function handleMessage(e) {
    let response = JSON.parse(e.data);
    if (response.type == "error") {
        displayJsonResponse("Server responded with error: " + response);
    } else if (response.type == "fetch") {
        if (response.messages.length > 0) {
            display("Fetched messages: ");
            displayJsonResponse(response);
        }
    } else if (response.type == "ownDetails") {
        display("ownDetails response: ");
        displayJsonResponse(response);
        ourAddress = response.address;
        display("Our address is:  " + ourAddress + ", we will now send messages to ourself.");
    }
}

function sendMessageToMixnet(connection) {
    var sendText = document.getElementById("sendtext").value;
    var message = {
        type: "send",
        message: sendText,
        recipient_address: ourAddress
    }
    display("Sending message to mixnet:");
    displayJsonSend(message);
    connection.send(JSON.stringify(message));
}

function sendOwnDetailsRequest(connection) {
    var ownDetails = {
        type: "ownDetails"
    }

    display("Sending a request for our own details:")
    displayJsonSend(ownDetails);
    connection.send(JSON.stringify(ownDetails));
}

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
    document.getElementById("output").innerHTML += "<p style='color: blue;'>" + JSON.stringify(message) + "</p >";
}

function displayJsonResponse(message) {
    document.getElementById("output").innerHTML += "<p style='color: green;'>" + JSON.stringify(message) + "</p >";
}

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
