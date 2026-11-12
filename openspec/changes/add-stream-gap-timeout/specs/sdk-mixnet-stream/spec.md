# sdk-mixnet-stream Spec Delta

## ADDED Requirements

### Requirement: Opt-in gap timeout for loss-tolerant message streams

A stream SHALL deliver frames strictly in sequence order unless a gap timeout is set for it. While a gap timeout is set, when frames are buffered behind a missing next sequence number and that number has been missing for at least the timeout, the router SHALL skip to the lowest buffered sequence number and SHALL deliver `StreamFailure::DataLoss` through the stream's ordered failure channel before the frames that follow, with the same delivery semantics as existing stream failures. The check SHALL run when frames arrive. Each missing sequence number SHALL have its own timer, so a gap that fills before the timeout causes no loss and does not shorten the wait for a later gap. A frame that arrives after its sequence number was skipped SHALL be dropped. The gap timeout SHALL be off by default; the overflow skip at `MAX_REORDER_BUFFER_BYTES` SHALL apply either way. The IP packet router stream (`IpMixStream`) SHALL set a gap timeout of 1.5 seconds.

#### Scenario: A lost frame no longer stalls the IPR tunnel

- **WHEN** a reply frame on an `IpMixStream` is lost and later frames keep arriving
- **THEN** once the missing frame has been missing for 1.5 seconds, the next arriving frame triggers a skip, `recv()` yields one `DataLoss`, and the buffered frames are delivered

#### Scenario: Byte streams keep strict ordering

- **WHEN** a frame is lost on a stream with no gap timeout and the buffer stays under the overflow cap
- **THEN** later frames stay buffered and nothing is delivered past the gap

#### Scenario: A gap that fills in time is not loss

- **WHEN** a missing frame arrives before the gap timeout
- **THEN** all frames are delivered in order with no `DataLoss`, and a later gap starts its own timer
