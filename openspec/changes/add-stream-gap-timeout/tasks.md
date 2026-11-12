# Tasks: Add Stream Gap Timeout

## 1. Stream router

- [x] 1.1 Add `gap_timeout: Option<Duration>` (default `None`) and `gap_since: Option<(u32, Instant)>` to `StreamEntry`
- [x] 1.2 In `send_to_stream`, before the overflow check: when a gap timeout is set and later frames wait behind a missing `next_seq` for at least the timeout, send `StreamFailure::DataLoss` and skip to the lowest buffered seq; restart the timer for each new gap
- [x] 1.3 Add `StreamMap::set_gap_timeout` and crate-internal `MixnetStream::set_gap_timeout`

## 2. IPR tunnel

- [x] 2.1 Set a 1.5 s gap timeout on the stream opened in `IpMixStream::connect`

## 3. Tests

- [x] 3.1 A stalled gap is skipped after the timeout, the loss is reported in order, and the late frame is dropped
- [x] 3.2 Without a gap timeout, a gap still holds later frames
- [x] 3.3 Each gap gets its own timer: a gap that fills before the timeout causes no loss, and the next gap does not inherit its start time
