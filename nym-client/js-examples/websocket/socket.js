display("Initialising...");

var port = '9001' // client websocket listens on 9001 by default, change if yours is different
var connection = new WebSocket('ws://127.0.0.1:' + port);

// When the connection is open, send some data to the server
connection.onopen = function () {
    display('Opened connection to Nym client websocket.');
};

// Log errors
connection.onerror = function (error) {
    display('WebSocket Error ' + error);
};

// Log messages from the server
connection.onmessage = function (e) {
    let response = JSON.parse(e.data);
    if (response.type == "error") {
        display('Server responded with error: ' + response.message);
    } else if (response.type == "fetch") {
        displayFetch(response.messages);
    }
};

function display(message) {
    document.getElementById("output").innerHTML = message;
}

function displayFetch(message) {
    document.getElementById("fetch").innerHTML = message;
}

display("Nym client demo started.");

var sequenceNum = 0;

// window.setInterval(function () {
//     var message = {
//         type: "send",
//         message: "FOOMP " + sequenceNum,
//         recipient_address: "2ub7f2s5en4Pn2nhY69uyWqGSMLZwhtPASjePq4gLxQs"
//     }
//     connection.send(JSON.stringify(message));
//     display("Sent message: " + message.message + " to " + message.recipient_address);
//     sequenceNum += 1;
// }, 500);

window.setInterval(function () {
    var message = {
        type: "fetch",
    }
    // displayFetch("fetching...");
    connection.send(JSON.stringify(message));
    sequenceNum += 1;
}, 1000);