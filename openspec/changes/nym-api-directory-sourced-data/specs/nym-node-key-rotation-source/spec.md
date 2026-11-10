## ADDED Requirements

### Requirement: Key rotation information is read from the chain first

nym-node SHALL read the interval and the key rotation state from the mixnet contract to learn the current epoch and key rotation, and SHALL ask nym-api for them only when the chain read fails. This SHALL hold wherever the node needs them: when it sets up key rotation at startup, in its running key rotation controller, and where it determines the current rotation id (initialisation, every start, the sphinx key reset command and config migration). The chain read SHALL go through the node's shared chain client. The commands that run without a node (initialisation, the sphinx key reset command and config migration) SHALL build that same client for the read, so that the node has one way of reading the chain.

#### Scenario: The node starts with every nym-api unreachable
- **WHEN** a node starts while its chain connection works and no nym-api answers
- **THEN** it reads the key rotation information from the chain, sets up key rotation and starts

#### Scenario: nym-api is the fallback
- **WHEN** the chain read of the key rotation information fails and a nym-api answers
- **THEN** the node uses the nym-api's answer and logs that it fell back

#### Scenario: The key reset command reads the chain
- **WHEN** the sphinx key reset command runs while the chain connection works and no nym-api answers
- **THEN** it takes the current rotation id from the chain and resets the keys

### Requirement: A stuck epoch slows the controller and never blocks startup

An epoch SHALL count as stuck when the time since its start exceeds its duration by more than 20% of that duration. The running key rotation controller, on finding the epoch stuck, SHALL log a warning and check again after 240 seconds without taking any key rotation action. Determining the current rotation id MUST NOT fail because the epoch is stuck: it SHALL return the rotation id of the epoch the source reports. The controller MUST NOT compare that rotation id with one derived from its own clock.

#### Scenario: A stuck epoch is reported by the controller
- **WHEN** the epoch lasts one hour and the controller finds it 15 minutes past its nominal end
- **THEN** it logs that the epoch is stuck and checks again after 240 seconds

#### Scenario: An overdue epoch within the threshold is not stuck
- **WHEN** the epoch lasts one hour and the controller finds it 5 minutes past its nominal end
- **THEN** it does not report a stuck epoch and checks again after 30 seconds

#### Scenario: A node starts during a stall
- **WHEN** a node is started while the epoch is 40 minutes past its nominal end
- **THEN** it takes the rotation id of the on-chain epoch and starts

### Requirement: The node does not ask nym-api to refresh its description

nym-node MUST NOT post to nym-api's describe refresh endpoint, at startup or at any other time, since nym-api reads node data from the directory.

#### Scenario: Startup sends no refresh request
- **WHEN** a node starts and publishes its directory entries
- **THEN** it sends no request to `/v1/nym-nodes/refresh-described` on any nym-api
