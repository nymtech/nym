function websocketUrl() {
  if ($(location).attr("href").startsWith("http://localhost")) {
    return "ws://localhost:1648";
  } else if ($(location).attr("href").startsWith("http://qa-explorer")) {
    return "ws://qa-explorer.nymtech.net:1648";
  } else {
    return "wss://testnet-explorer.nymtech.net";
  }
}

function parseVersion(str) {
  if (typeof (str) != 'string') { return false; }

  var arr = str.split('.');

  // parse int or default to 0
  var maj = parseInt(arr[0]) || 0;
  var min = parseInt(arr[1]) || 0;
  var rest = parseInt(arr[2]) || 0;
  return {
    major: maj,
    minor: min,
    patch: rest
  }
}

function getTopology() {
  console.log("Getting topology...");
  var topologyUrl = "/downloads/topology.json";
  $.ajax({
    type: 'GET',
    url: topologyUrl,
    success: function (data) {
      createMixnodeCount(data.mixNodes.length);
      createValidatorCount(data.validators.validators.length);
      createBlockHeight(data.validators.block_height);
      createDisplayTable(data);
      updateNodesStatus();
    }
  });
}

function createDisplayTable(data) {
  createMixnodeRows(data.mixNodes);
  createValidatorRows(data.validators.validators);
  createGatewayRows(data.gateways);
}

function clearStatus(element) {
  element.removeAttribute("active")
  element.removeAttribute("positive")
  element.removeAttribute("intermediary")
  element.removeAttribute("negative")
}

function setNodeStatus(dotWrapper, reportData) {
  let statusIndicator = dotWrapper.children[0];
  clearStatus(statusIndicator)

  if (reportData == undefined || reportData == null) {
    dotWrapper.setAttribute("title", "no data available")
    return
  }


  if (reportData.mostRecentIPV4 && reportData.mostRecentIPV6 && reportData.lastHourIPV4 > 50 && reportData.lastHourIPV6 > 50) {
    statusIndicator.setAttribute("positive", "")
  } else if (reportData.mostRecentIPV4 || reportData.mostRecentIPV6) {
    statusIndicator.setAttribute("intermediary", "")
  } else {
    statusIndicator.setAttribute("negative", "")
  }

  let newTooltip = `\n
  IPv4 routable: ${reportData.mostRecentIPV4}\n
  Last hour IPv4: ${reportData.lastHourIPV4}%\n
  IPv6 routable: ${reportData.mostRecentIPV6}\n
  Last hour IPv6: ${reportData.lastHourIPV6}%\n
  `
  dotWrapper.setAttribute("title", newTooltip)
}

function dealWithStatusReport(report) {
  let reportMap = new Map();
  report.forEach(reportData => {
    reportMap.set(reportData.pubKey, reportData)
  })

  let allWrappers = document.getElementsByClassName('statusDot');
  for (let statusWrapper of allWrappers) {
    let mapEntry = reportMap.get(statusWrapper.getAttribute('pubkey'))
    setNodeStatus(statusWrapper, mapEntry)
  }
}

function updateNodesStatus() {
  console.log("updating node statuses!")

  const reportUrl = "/downloads/mixmining.json";
  fetch(reportUrl, {
    method: 'GET'
  })
    .then((response) => response.json())
    .then((data) => dealWithStatusReport(data.report)).catch((err) => {
      console.log("getting full mixmining report failed - ", err)
    })
}

function makeStatusDot(nodePubKey) {
  let statusText = "pending..."

  let dotWrapper = document.createElement("div");
  dotWrapper.setAttribute('id', `dotWrapper${nodePubKey}`)
  dotWrapper.setAttribute('pubkey', nodePubKey)
  dotWrapper.setAttribute('style', 'text-align: center')
  dotWrapper.setAttribute('data-toggle', 'tooltip')
  dotWrapper.setAttribute('data-placement', 'right')
  dotWrapper.setAttribute('title', statusText)
  dotWrapper.classList.add('statusDot')

  let dot = document.createElement("status-indicator");
  dotWrapper.appendChild(dot);

  return dotWrapper;
}

function outdatedStatus(nodePubKey) {
  let statusText = "Out of date"

  let dotWrapper = document.getElementById(`dotWrapper${nodePubKey}`);
  dotWrapper.classList.remove('statusDot')
  let statusIndicator = dotWrapper.children[0];
  clearStatus(statusIndicator);
  statusIndicator.setAttribute("active", "")

  dotWrapper.setAttribute("title", statusText)
}


function setGatewayStatusDot(nodePubKey) {
  let statusText = "Data not available..."
  let dotWrapper = document.getElementById(`dotWrapper${nodePubKey}`);
  dotWrapper.classList.remove('statusDot')
  let statusIndicator = dotWrapper.children[0];
  clearStatus(statusIndicator);
  statusIndicator.setAttribute("active", "")

  dotWrapper.setAttribute("title", statusText)
}

function createMixnodeCount(mixNodeCount) {
  // no need to sanitize numbers (count is obtained via .lengnth attribute of an array)
  $('#mixnodes-count').text(mixNodeCount);
}

function createValidatorCount(validatorCount) {
  // no need to sanitize numbers (count is obtained via .lengnth attribute of an array)
  $('#validators-count').text(validatorCount);
}

function createBlockHeight(blockHeight) {
  let purifiedHeight = DOMPurify.sanitize(blockHeight)
  if (purifiedHeight.length === 0) {
    purifiedHeight = 0
  }

  $('#block-height').text(purifiedHeight);
}

function compareNodes(node1, node2) {
  if (node1.reputation < node2.reputation) {
    return 1
  } else if (node1.reputation > node2.reputation) {
    return -1
  } else {
    if (node1.version < node2.version) {
      return 1
    } else if (node1.version > node2.version) {
      return -1
    } else {
      if (node1.layer < node2.layer) {
        return 1
      } else {
        return -1
      }
    }
  }
}

function createMixnodeRows(mixNodes) {
  mixNodes.sort(compareNodes)

  const currentUnixTime = new Date().getTime() * 1000000;

  mixNodes.forEach(node => {
    // because javascript works in mysterious ways, if you sanitize "0", it will return ""
    let purifiedRep = DOMPurify.sanitize(node.reputation)
    if (purifiedRep.length === 0) {
      purifiedRep = 0
    }

    let purifiedVersion = DOMPurify.sanitize(node.version)

    var $tr = $('<tr>').append(
      $('<input type="hidden" id="prev-timestamp-' + node.identityKey + '" value="' + currentUnixTime + '"> '),
      $('<td>').html(makeStatusDot(node.identityKey)),
      $('<td>').text(purifiedRep),
      $('<td>').text(purifiedVersion),
      $('<td>').text(DOMPurify.sanitize(node.identityKey)),
      $('<td>').text(DOMPurify.sanitize(node.sphinxKey)),
      $('<td>').text(DOMPurify.sanitize(node.location)),
      $('<td>').text(DOMPurify.sanitize(node.mixHost)),
      $('<td>').text(DOMPurify.sanitize(node.layer)),
      $('<td id="' + "received-" + DOMPurify.sanitize(node.identityKey) + '">').text("0"),
      $('<td id="' + "sent-" + DOMPurify.sanitize(node.identityKey) + '">').text("0")
    ).appendTo('#mixnodes-list');

    let version = parseVersion(purifiedVersion)

    if (version.major >= 1 || version.minor >= 10 || (version.minor == 9 && version.patch >= 2)) {
      makeStatusDot(node.identityKey);
    } else {
      outdatedStatus(node.identityKey);
    }
  })
}

function createGatewayRows(gatewayNodes) {
  gatewayNodes.forEach(node => {
    // because javascript works in mysterious ways, if you sanitize "0", it will return ""
    let purifiedRep = DOMPurify.sanitize(node.reputation)
    if (purifiedRep.length === 0) {
      purifiedRep = 0
    }
    var $tr = $('<tr>').append(
      $('<input type="hidden" id="prev-timestamp-' + node.pubKey + '" value="' + node.timestamp + '"> '),
      $('<td>').html(makeStatusDot(node.identityKey)),
      $('<td>').text(purifiedRep),
      $('<td>').text(DOMPurify.sanitize(node.version)),
      $('<td>').text(DOMPurify.sanitize(node.identityKey)),
      $('<td>').text(DOMPurify.sanitize(node.sphinxKey)),
      $('<td>').text(DOMPurify.sanitize(node.location)),
      $('<td>').text(DOMPurify.sanitize(node.mixHost)),
      $('<td>').text(DOMPurify.sanitize(node.clientsHost)),
    ).appendTo('#gatewaynodes-list');

    setGatewayStatusDot(node.identityKey);
  })
}

function createValidatorRows(validators) {
  validators.forEach(validator => {
    var $tr = $('<tr>').append(
      $('<td>').text(DOMPurify.sanitize(validator.address)),
      $('<td>').text(DOMPurify.sanitize(validator.pub_key)),
      $('<td>').text(DOMPurify.sanitize(validator.proposer_priority)),
      $('<td>').text(DOMPurify.sanitize(validator.voting_power))
    ).appendTo('#validator-list');
  })
}

function connectWebSocket() {
  var conn;
  var url;
  url = websocketUrl() + "/ws";
  console.log("connecting to: " + url);
  conn = new WebSocket(url);
  conn.onmessage = function (evt) {
    processMessage(evt);
  };
}

function processMessage(evt) {
  var messages = evt.data.split('\n');
  for (var i = 0; i < messages.length; i++) {
    var msg = jQuery.parseJSON(messages[i]);
    prevTimestamp = updateTimeStampStorage(msg);

    timeDiff = (msg.timestamp - prevTimeStamp) / 1000000000;
    displayReceivedPackets(msg, timeDiff);
    displaySentPackets(msg, timeDiff);
  }
}

function displaySentPackets(msg, timeDiff) {
  var sentCell = "#sent-" + DOMPurify.sanitize(msg.pubKey);
  var sent = 0;
  for (var key in msg.sent) {
    s = msg.sent[key];
    sent += s;
  }
  sentPerSecond = Math.floor(sent / timeDiff);
  let sentVal = DOMPurify.sanitize(sentPerSecond).length > 0 ? DOMPurify.sanitize(sentPerSecond) : "0";
  $(sentCell).html(sentVal);
}

function displayReceivedPackets(msg, timeDiff) {
  receivedPerSecond = Math.floor(msg.received / timeDiff);
  var recCell = "#received-" + DOMPurify.sanitize(msg.pubKey);
  let recVal = DOMPurify.sanitize(receivedPerSecond).length > 0 ? DOMPurify.sanitize(receivedPerSecond) : "0";
  $(recCell).html(recVal);
}

/* 
  Hahahaha this has to be the crappiest code I've written since learning to code.

  On the upside, it'll save a few weeks messing with React or Angular to do
  basically the same thing.
*/
function updateTimeStampStorage(msg) {
  // get the timestamp stored during the last loop
  prevTimeStamp = ($("#prev-timestamp-" + msg.pubKey).val())

  // store the current timestamp
  $('#prev-timestamp-' + msg.pubKey).val(msg.timestamp);

  // return the previous timestamp
  return prevTimeStamp;
}


document.addEventListener("DOMContentLoaded", function () {
  // update every minute
  // setInterval(updateNodesStatus, 60000);
  getTopology();
  connectWebSocket();
});

