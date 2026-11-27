# playground-speed-test Spec Delta

## ADDED Requirements

### Requirement: The traffic-shape simulator shows what the entry gateway sees

The documentation SHALL provide a client-only traffic-shape simulator that needs no mixnet connection. For a chosen traffic preset (Idle, Chat, Burst, Stream), it SHALL draw the packets that the application sends and the packets that the entry gateway receives over a 10 s window. It SHALL model the client send loop as client-core implements it: with Poisson pacing on, the main stream sends one queued real packet or one cover packet on each exponential tick (mean 20 ms); with Poisson pacing off, real packets leave when they arrive and the main stream sends no cover; the loop cover stream (mean 200 ms) runs on its own clock unless it is turned off. An observer view SHALL draw real and cover packets in one colour.

#### Scenario: Idle and busy look the same with Poisson pacing on

- **WHEN** Poisson pacing is on and the visitor switches between the Idle, Chat and Burst presets with the same seed
- **THEN** the gateway lane shows the same packet times for each preset, and only the colour view shows which packets are real

#### Scenario: Turning off the loop cover stream does not reveal traffic

- **WHEN** Poisson pacing is on and the loop cover stream is off
- **THEN** the gateway lane still shows a packet on every main-stream tick, because the main stream fills empty ticks with cover

#### Scenario: Poisson pacing off reveals the traffic shape

- **WHEN** Poisson pacing is off and the preset is Burst
- **THEN** the gateway lane in observer view shows the burst at the time the application sent it

#### Scenario: Over-budget traffic builds a queue

- **WHEN** Poisson pacing is on and the preset is Stream (80 pkt/s)
- **THEN** the queue-depth chart rises over the window and packets are still queued at the end of it

#### Scenario: The same seed gives the same picture

- **WHEN** the simulator runs twice with the same seed and the same inputs
- **THEN** both runs give the same packet list

### Requirement: The model constants match the Rust sources

The speed-test model SHALL use the client-core defaults (main-stream mean gap, loop-cover mean gap, mean mix-hop delay) and the IPR return flush interval. A unit test SHALL read these values from the Rust source files and compare them with the model constants. The docs test workflow SHALL run when those Rust files change.

#### Scenario: A changed Rust default fails the docs test

- **WHEN** `DEFAULT_MESSAGE_STREAM_AVERAGE_DELAY`, `DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY`, `DEFAULT_AVERAGE_PACKET_DELAY` or `BUFFER_TIMEOUT` changes in Rust and the model is not updated
- **THEN** the vitest suite fails and names the constant

### Requirement: The throughput ramp reports the knee and a capacity estimate

The playground SHALL provide a throughput ramp test over its own MixWebSocket connection on the existing tunnel. The test SHALL send fixed-size echo messages at a series of increasing offered loads in bytes per second, and wait for the echoes of one step (or a drain timeout) before it starts the next step. For each step it SHALL fit RTT against send time and report the slope. It SHALL report the first step with a slope above 0.1 as the knee, and the capacity estimate `offered / (1 + slope)` from that step, next to the raw send budget and the TCP window limit at the measured base RTT. It SHALL NOT state a predicted knee or a count of Sphinx packets per message.

#### Scenario: RTT stays flat below the limit

- **WHEN** the offered load of a step is well below the path capacity
- **THEN** the fitted slope of that step is close to zero

#### Scenario: RTT climbs above the limit

- **WHEN** the offered load of a step is above the path capacity
- **THEN** the fitted slope of that step is above 0.1, the page reports it as the knee, and the capacity estimate is lower than the offered load of that step

#### Scenario: No knee in the range

- **WHEN** no step has a slope above 0.1
- **THEN** the page reports that there is no knee up to the highest offered load, and shows no capacity estimate

#### Scenario: A backlog does not carry into the next step

- **WHEN** a step ends with echoes still in flight
- **THEN** the test waits until those echoes arrive or the drain timeout passes before it sends the first message of the next step

#### Scenario: The ramp stops after the knee

- **WHEN** a step is the first step past the knee, more than half of its echoes are lost, or its median RTT is above 5 s
- **THEN** the test sends no further steps and says why it stopped

### Requirement: The RTT histogram compares the measured spread with the model

The playground SHALL provide an RTT histogram test that sends echo messages one at a time, with a random gap between an echo and the next message. It SHALL plot a histogram of the measured RTTs and overlay a model of the mixing and pacing delays: six exponential hop delays (mean 15 ms), one uniform IPR flush wait (0 to 20 ms) and, with Poisson pacing on, one exponential forward send-slot wait (mean 20 ms). The model SHALL be shifted by a fitted offset equal to the measured 5th percentile minus the model 5th percentile. The page SHALL show the offset and SHALL say that the measured spread is expected to be equal to the model or wider.

#### Scenario: The overlay uses the measured bins

- **WHEN** a histogram run finishes
- **THEN** the model curve is drawn over the same bins as the measured histogram, and the page shows min, p50, p95, max and the fitted offset

#### Scenario: Poisson pacing off removes only the slot wait from the model

- **WHEN** the tunnel was set up with Poisson pacing off
- **THEN** the model curve has no send-slot wait and keeps the six hop delays and the flush wait

### Requirement: Runs can be compared across a page reload

Because the wasm tunnel is one-shot, the playground SHALL store finished speed-test runs and the tunnel options that were in use in `sessionStorage`, and SHALL offer a button that stores pending tunnel options (for example Poisson pacing off) and reloads the page. After the reload, the playground SHALL fill in its toggles from the pending options, clear them, and tell the visitor to set up the tunnel, without connecting by itself. All storage reads SHALL happen after the first render. The charts SHALL draw the most recent stored runs of each kind, each with a label from its options. A storage failure SHALL NOT stop a test from running.

#### Scenario: A comparison survives the reload

- **WHEN** the visitor finishes a ramp with Poisson pacing on, clicks "Run again with Poisson off", sets up the tunnel and runs the ramp again
- **THEN** the ramp chart shows both runs, each labelled with its Poisson setting

#### Scenario: The label shows the options that were used

- **WHEN** the visitor changes the Poisson checkbox after the tunnel is up and then runs a test
- **THEN** the run is labelled with the options that `setupMixTunnel` received, not with the current checkbox

#### Scenario: Pending options apply once

- **WHEN** the page has applied the pending options and the visitor reloads again
- **THEN** the toggles start at their defaults

#### Scenario: Storage is not available

- **WHEN** `sessionStorage` throws on read or write
- **THEN** the tests run as normal and the page says that the comparison is not available

### Requirement: The speed tests do not interfere with other traffic

The speed-test section SHALL use its own MixWebSocket connection and its own message handler. Probe frames SHALL be binary, with an 8-byte header of a magic tag and a sequence number that is unique across runs on the socket. The handler SHALL ignore any frame that is not a probe or whose sequence number is not outstanding. While a speed test runs or drains, the playground SHALL disable every other control that sends traffic through the tunnel or changes it, and a speed test SHALL NOT start while another section is busy.

#### Scenario: Other traffic cannot start during a run

- **WHEN** a ramp or histogram run is in progress or draining
- **THEN** Disconnect and the DNS, GET, WebSocket, stress and download controls are disabled until the drain ends

#### Scenario: Frames that are not outstanding probes are ignored

- **WHEN** the echo server sends a text greeting, or an echo arrives from a cancelled run
- **THEN** the handler ignores the frame and does not count it as an echo or a loss

#### Scenario: The echo burst still works after a speed test

- **WHEN** a ramp run has finished and drained and the visitor runs the existing echo burst
- **THEN** the burst reports its own messages only, with no mismatches caused by the speed test
