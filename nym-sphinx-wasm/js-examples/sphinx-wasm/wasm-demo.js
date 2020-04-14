display("Initialising...");

var port = '1793' // gateway websocket listens on 1793 by default, change if yours is different
var connection = new WebSocket('ws://127.0.0.1:' + port);

// Open a connection and display a status message.
connection.onopen = function () {
    display('Opened connection to Nym gateway websocket.');
};

// Log errors
connection.onerror = function (error) {
    display('WebSocket Error ' + error);
};

function display(message) {
    document.getElementById("output").innerHTML = message;
}

var sequenceNum = 0;

window.setInterval(function () {
    var message = "FOOMP FOOMP FOOMP " + sequenceNum;
    connection.send(message);
    display("Sent: " + message);
    sequenceNum += 1;
}, 500);
