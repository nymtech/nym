# Tasks: Add Stream Gap Timeout

## 1. Stream router

- [x] 1.1 Add `gap_timeout: Option<Duration>` (default `None`) and `arrived: BTreeMap<u32, Instant>` (arrival time per buffered frame, only while the timeout is set) to `StreamEntry`
- [x] 1.2 In `send_to_stream`, before the overflow check: while the oldest buffered frame (all wait behind a missing `next_seq`) has waited at least the timeout, send `StreamFailure::DataLoss`, skip to the lowest buffered seq and drain, so a burst of gaps is skipped in one pass
- [x] 1.3 Add `StreamMap::set_gap_timeout` and crate-internal `MixnetStream::set_gap_timeout`

## 2. IPR tunnel

- [x] 2.1 Set a 1.5 s gap timeout on the stream opened in `IpMixStream::connect`

## 3. Tests

- [x] 3.1 A stalled gap is skipped after the timeout, the loss is reported in order, and the late frame is dropped
- [x] 3.2 Without a gap timeout, a gap still holds later frames
- [x] 3.3 A gap that fills before the timeout causes no loss, and a later gap is timed from the frames behind it
- [x] 3.4 A gap exposed by draining is timed from when the frames behind it arrived
- [x] 3.5 A burst of gaps whose waiting frames are all older than the timeout is skipped in one pass
- [x] 3.6 A gap is timed from the oldest frame waiting behind it, even when a lower seq arrived later
