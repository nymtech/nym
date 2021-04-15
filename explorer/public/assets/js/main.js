function websocketUrl() {
  if ($(location).attr("href").startsWith("http://localhost")) {
    return "ws://localhost:1648";
  } else if ($(location).attr("href").startsWith("http://qa-explorer")) {
    return "ws://qa-explorer.nymtech.net:1648";
  } else if ($(location).attr("href").startsWith("http://nicenet-explorer")) {
    return "ws://nicenet-explorer.nymtech.net:1648";
  } else {
    return "wss://testnet-explorer.nymtech.net";
  }
}

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

// gets only active nodes
async function getActiveTopology() {
  console.log("Getting active topology...");
  const topologyUrl = "/downloads/topology_active.json";
  const response = await fetch(topologyUrl, { method: 'GET' })
  return await response.json()
}

// gets 'removed' topology, i.e. nodes that failed at some point and got slashed
async function getRemovedTopology() {
  console.log("Getting removed topology...");
  const topologyUrl = "/downloads/topology_removed.json";
  const response = await fetch(topologyUrl, { method: 'GET' })
  return await response.json()
}

// gets 'full' topology, i.e. all active + standby
async function getTopology() {
  console.log("Getting topology...");
  const topologyUrl = "/downloads/topology.json";

  const response = await fetch(topologyUrl, { method: 'GET' })
  return await response.json()
}

async function dealWithInitialTopology() {
  const fullTopology = await getTopology();
  const activeTopology = await getActiveTopology();
  const removedTopology = await getRemovedTopology();

  let activeMixes = new Set()
  activeTopology.mixNodes.forEach(activeMix => {
    activeMixes.add(activeMix.identityKey)
  })
  let activeGateways = new Set()
  activeTopology.gateways.forEach(activeGateway => {
    activeGateways.add(activeGateway.identityKey)
  })

  const standbyMixes = fullTopology.mixNodes.filter(mix => !activeMixes.has(mix.identityKey))
  const standbyGateways = fullTopology.gateways.filter(gateway => !activeGateways.has(gateway.identityKey))
  // lets ignore removed gateways for time being (at least until network monitor actually sends packets to them)

  createMixnodeCount(`${activeTopology.mixNodes.length} + ${standbyMixes.length} standby`)
  createValidatorCount(fullTopology.validators.validators.length || 0);
  createBlockHeight(fullTopology.validators.block_height);

  createDisplayTable(activeTopology, standbyMixes, standbyGateways, removedTopology.mixNodes)
}

function createDisplayTable(activeTopology, standbyMixes, standbyGateways, removedMixes) {
  createActiveMixnodeRows(activeTopology.mixNodes);
  createStandbyMixnodeRows(standbyMixes);
  createRemovedMixnodeRows(removedMixes);
  createValidatorRows(activeTopology.validators.validators);
  createGatewayRows(activeTopology.gateways);
  createStandbyGatewayRows(standbyGateways);
}

function clearStatus(element) {
  element.removeAttribute("active")
  element.removeAttribute("positive")
  element.removeAttribute("intermediary")
  element.removeAttribute("negative")
}

function setNodeStatus(dotWrapper, reportData) {
  // don't do anything to removed nodes
  if (dotWrapper.children[0].hasAttribute("removed")) {
    return
  }

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

async function updateNodesStatus() {
  console.log("updating node statuses!")

  const reportUrl = "/downloads/mixmining.json";
  const response = await fetch(reportUrl, { method: 'GET' });
  const report = await response.json();
  dealWithStatusReport(report.report)
}

function makeInitialStatusDot(nodePubKey) {
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

function removedStatus(nodePubKey) {
  let statusText = "Removed due to being outdated or providing bad-quality service"

  let dotWrapper = document.getElementById(`dotWrapper${nodePubKey}`);
  dotWrapper.classList.remove('statusDot')
  let statusIndicator = dotWrapper.children[0];
  clearStatus(statusIndicator);
  statusIndicator.setAttribute("removed", "")

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
  // no need to sanitize numbers (count is obtained via .length attribute of an array)
  $('#mixnodes-count').text(mixNodeCount);
}

function createValidatorCount(validatorCount) {
  // no need to sanitize numbers (count is obtained via .length attribute of an array)
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

function createActiveMixnodeRows(mixNodes) {
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
      $('<td>').html(makeInitialStatusDot(node.identityKey)),
      $('<td>').text(purifiedRep),
      $('<td>').text(purifiedVersion),
      $('<td>').text(DOMPurify.sanitize(node.identityKey)),
      $('<td>').text(DOMPurify.sanitize(node.sphinxKey)),
      $('<td>').text(DOMPurify.sanitize(node.location)),
      $('<td>').text(DOMPurify.sanitize(node.mixHost)),
      $('<td>').text(DOMPurify.sanitize(node.layer)),
      $('<td id="' + "received-" + DOMPurify.sanitize(node.identityKey) + '">').text("0"),
      $('<td id="' + "sent-" + DOMPurify.sanitize(node.identityKey) + '">').text("0")
    ).appendTo('#active-mixnodes-list');
  })
}

function createStandbyMixnodeRows(mixNodes) {
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
      $('<td>').html(makeInitialStatusDot(node.identityKey)),
      $('<td>').text(purifiedRep),
      $('<td>').text(purifiedVersion),
      $('<td>').text(DOMPurify.sanitize(node.identityKey)),
      $('<td>').text(DOMPurify.sanitize(node.sphinxKey)),
      $('<td>').text(DOMPurify.sanitize(node.location)),
      $('<td>').text(DOMPurify.sanitize(node.mixHost)),
      $('<td>').text(DOMPurify.sanitize(node.layer)),
      $('<td id="' + "received-" + DOMPurify.sanitize(node.identityKey) + '">').text("0"),
      $('<td id="' + "sent-" + DOMPurify.sanitize(node.identityKey) + '">').text("0")
    ).appendTo('#standby-mixnodes-list');
  })
}

function createRemovedMixnodeRows(mixNodes) {
  mixNodes.sort(compareNodes)

  mixNodes.forEach(node => {
    // because javascript works in mysterious ways, if you sanitize "0", it will return ""
    let purifiedRep = DOMPurify.sanitize(node.reputation)
    if (purifiedRep.length === 0) {
      purifiedRep = 0
    }

    let purifiedVersion = DOMPurify.sanitize(node.version)
    let purifiedIdentity = DOMPurify.sanitize(node.identityKey)
    var $tr = $('<tr>').append(
      $('<td>').html(makeInitialStatusDot(node.identityKey)),
      $('<td>').text(purifiedRep),
      $('<td>').text(purifiedVersion),
      $('<td>').text(purifiedIdentity),
      $('<td>').text(DOMPurify.sanitize(node.sphinxKey)),
      $('<td>').text(DOMPurify.sanitize(node.location)),
      $('<td>').text(DOMPurify.sanitize(node.mixHost)),
      $('<td>').text(DOMPurify.sanitize(node.layer)),
    ).appendTo('#removed-mixnodes-list');

    removedStatus(purifiedIdentity)
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
      $('<td>').html(makeInitialStatusDot(node.identityKey)),
      $('<td>').text(purifiedRep),
      $('<td>').text(DOMPurify.sanitize(node.version)),
      $('<td>').text(DOMPurify.sanitize(node.identityKey)),
      $('<td>').text(DOMPurify.sanitize(node.sphinxKey)),
      $('<td>').text(DOMPurify.sanitize(node.location)),
      $('<td>').text(DOMPurify.sanitize(node.mixHost)),
      $('<td>').text(DOMPurify.sanitize(node.clientsHost)),
    ).appendTo('#active-gatewaynodes-list');

    setGatewayStatusDot(node.identityKey);
  })
}

function createStandbyGatewayRows(gatewayNodes) {
  gatewayNodes.forEach(node => {
    // because javascript works in mysterious ways, if you sanitize "0", it will return ""
    let purifiedRep = DOMPurify.sanitize(node.reputation)
    if (purifiedRep.length === 0) {
      purifiedRep = 0
    }
    var $tr = $('<tr>').append(
      $('<input type="hidden" id="prev-timestamp-' + node.pubKey + '" value="' + node.timestamp + '"> '),
      $('<td>').html(makeInitialStatusDot(node.identityKey)),
      $('<td>').text(purifiedRep),
      $('<td>').text(DOMPurify.sanitize(node.version)),
      $('<td>').text(DOMPurify.sanitize(node.identityKey)),
      $('<td>').text(DOMPurify.sanitize(node.sphinxKey)),
      $('<td>').text(DOMPurify.sanitize(node.location)),
      $('<td>').text(DOMPurify.sanitize(node.mixHost)),
      $('<td>').text(DOMPurify.sanitize(node.clientsHost)),
    ).appendTo('#standby-gatewaynodes-list');

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

function handleMetricsSocket() {
  connectWebSocket()

  function connectWebSocket() {
    var conn;
    var url;
    url = websocketUrl() + "/ws";
    console.log("connecting to: " + url);
    conn = new WebSocket(url);
    conn.onmessage = function (evt) {
      processWebSocketMessage(evt);
    };
  }

  function processWebSocketMessage(evt) {
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
  main()
});

async function main() {
  await dealWithInitialTopology();
  handleMetricsSocket();

  while (true) {
    await updateNodesStatus()
    await sleep(60000)
  }
}
