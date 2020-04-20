// import { MDCRipple } from '@material/ripple/index';
// const ripple = new MDCRipple(document.querySelector('.foo-button'));

console.log('hello world from dave');

async function main() {
    console.log("Hello world from main()");
    display("Initialising...");

    var port = '9001' // client websocket listens on 9001 by default, change if yours is different
    var localClientUrl = "ws://127.0.0.1:" + port;

    var connection = await connectWebsocket(localClientUrl).then(function (c) {
        return c;
    }).catch(function (err) {
        console.log("Websocket ERROR: " + err);
    })

    connection.onmessage = function (e) {
        handleMessage(e);
    };
    var ownDetails = {
        type: "ownDetails"
    }

    connection.send(JSON.stringify(ownDetails));
}

console.log("before main");

// Let's get this private party started!
main();


console.log("after main");

function handleMessage(e) {
    let response = JSON.parse(e.data);
    if (response.type == "error") {
        display("Server responded with error: " + response.message);
    } else if (response.type == "fetch") {
        display(response.messages);
    } else {
        display("response: " + JSON.stringify(response));
    }
}

function display(message) {
    document.getElementById("output").innerHTML = message;
}

function print(name, obj) {
    console.log(name + ": " + JSON.stringify(obj));
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


    // setInterval(() => {
    //     var message = {
    //         type: "send",
    //         message: "FOOMP " + sequenceNum,
    //         recipient_address: "2ub7f2s5en4Pn2nhY69uyWqGSMLZwhtPASjePq4gLxQs"
    //     }
    //     connection.send(JSON.stringify(message));
    //     display("Sent message: " + message.message + " to " + message.recipient_address);
    //     sequenceNum += 1;
    // }, 3000);


// var sequenceNum = 0;

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

// window.setInterval(function () {
//     var message = {
//         type: "fetch",
//     }
//     // displayFetch("fetching...");
//     connection.send(JSON.stringify(message));
//     sequenceNum += 1;
// }, 1000);


