## MODIFIED Requirements

### Requirement: The loop's wake-up shortens to the debounce deadline

`watch::poll_timeout(tick: Duration, pending_in: Option<Duration>) -> Duration` SHALL be a
pure function of two `Duration`s — reading no clock — returning `tick` when nothing is
pending and otherwise the smaller of `tick` and the remaining window, floored at
**1 millisecond**.

The floor is load-bearing rather than cosmetic: a zero timeout returned every iteration would
make `run_loop` spin without bound if a watcher ever reported a pending batch it then declined
to yield. One millisecond converts an **unbounded** spin into a **bounded** one — roughly a
thousand loop iterations a second, each of them a full `sync_detail`, draw, and
`normalise_scroll`. That is survivable long enough for a reader to notice and quit, not free;
the floor exists so a faulty `FsEvents` degrades to a hot pane rather than to a hung one. The
correct case never reaches it, because the next iteration's `drain` yields the batch and
`pending_in` returns `None` again.

`agent-polling` adds a **second** pending deadline beside the debounce's, and the loop has one
wait to serve both. `watch::soonest(a: Option<Duration>, b: Option<Duration>) -> Option<Duration>`
SHALL therefore be a pure function of two optional `Duration`s — reading no clock — returning
`None` when both are `None`, the present one when exactly one is present, and the smaller when
both are. `run_loop` SHALL wait
`poll_timeout(tick, soonest(live.fs.pending_in(), live.agents.pending_in()))`.

`poll_timeout`'s own signature SHALL NOT change, and the minimum SHALL NOT be taken inline in
`src/ui/driver.rs`. Both are deliberate. Keeping `poll_timeout` at two arguments leaves every
landed scenario and every landed assertion on it untouched, so a change about agents does not
re-open the debounce's contract; and a named function is directly asserted over all four
combinations of present and absent, whereas an inline `min` in the driver would be exercised
only through frame counts that pass whichever way it was written. `soonest` lives in
`src/watch.rs` beside `poll_timeout` because that is its only consumer and the two are one
piece of arithmetic; it names nothing about agents and nothing about the filesystem.

`ui::driver::TICK` SHALL remain 250 milliseconds. The worst-case latency from a file write to
a corrected frame is therefore `TICK` (discovering the event) plus `DEBOUNCE` (coalescing it)
plus the CLI's own 200–400ms — the post-debounce half of which this function removes by
waking the loop exactly at the window's end rather than at the next tick.

#### Scenario: The timeout is the tick when nothing is pending

- **WHEN** `poll_timeout(Duration::from_millis(250), None)` is called
- **THEN** it returns `Duration::from_millis(250)`

#### Scenario: The timeout is the remaining window when that is shorter

- **WHEN** `poll_timeout(250ms, Some(90ms))` and `poll_timeout(250ms, Some(400ms))` are called
- **THEN** they return `90ms` and `250ms` respectively — the smaller of the two, never the
  pending value unclamped
- **AND** `poll_timeout(250ms, Some(0ms))` returns `1ms`, never `0ms`, so a watcher reporting
  a pending batch it does not yield cannot spin the loop

#### Scenario: One tick serves two pollers

- **WHEN** `soonest` is called with `(None, None)`, `(Some(90ms), None)`, `(None, Some(40ms))`,
  `(Some(90ms), Some(40ms))`, and `(Some(40ms), Some(90ms))`
- **THEN** it returns `None`, `Some(90ms)`, `Some(40ms)`, `Some(40ms)`, and `Some(40ms)`
  respectively — commutative, and never the larger of two present values
- **AND** `poll_timeout(250ms, soonest(Some(900ms), Some(120ms)))` is `120ms`, so an agent poll
  becoming due inside the tick shortens the wait exactly as a debounce deadline does
- **AND** `poll_timeout(250ms, soonest(Some(900ms), Some(3s)))` is `250ms`, so a deadline
  further away than the tick never lengthens the wait
- **AND** `soonest` reads no clock: it is asserted with literal `Duration` values and takes no
  `Instant`, so no view test can reach a clock through it
