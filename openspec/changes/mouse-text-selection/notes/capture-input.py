#!/usr/bin/env python3
"""Log what the TERMINAL sends a TUI, to settle what happens during a drag.

Every probe so far has measured what an application *writes*. The open question
is the other direction: while you drag to select inside Copilot, does the
terminal deliver mouse reports to it, or does it keep the drag for itself?

    python3 notes/capture-input.py copilot

Use the program, DRAG TO SELECT SOME TEXT, then quit it. The summary says
whether any mouse report arrived while you were dragging.

If reports DID arrive, the terminal reports and selects at the same time, and
every "no" recorded by probe-altscreen.sh is a probe bug.
If reports did NOT arrive, the terminal stopped reporting for that gesture, and
the reason is something other than the mode set -- which is the whole mystery.
"""
import os
import pty
import re
import sys
import time

if len(sys.argv) < 2:
    sys.exit("usage: capture-input.py <program> [args...]")

log_path = "/tmp/capture-input-%d.log" % os.getpid()
log = open(log_path, "wb")
events = []

SGR = re.compile(rb"\x1b\[<(\d+);(\d+);(\d+)([Mm])")


def stdin_read(fd):
    data = os.read(fd, 4096)
    if data:
        log.write(b"%.3f IN  %r\n" % (time.time(), data))
        log.flush()
        for m in SGR.finditer(data):
            events.append((time.time(), int(m.group(1)), int(m.group(2)),
                           int(m.group(3)), m.group(4).decode()))
    return data


def master_read(fd):
    return os.read(fd, 4096)


pty.spawn([sys.argv[1]] + sys.argv[2:], master_read, stdin_read)
log.close()

# Button encoding: low 2 bits are the button; 32 adds "motion"; 64 is the wheel.
press = [e for e in events if e[4] == "M" and e[1] < 32]
motion = [e for e in events if e[1] & 32 and e[1] < 64]
release = [e for e in events if e[4] == "m"]
wheel = [e for e in events if e[1] >= 64]

print("\n=== what the terminal sent %s ===" % sys.argv[1])
print("  button presses : %d" % len(press))
print("  DRAG MOTION    : %d   <-- the decisive number" % len(motion))
print("  releases       : %d" % len(release))
print("  wheel          : %d" % len(wheel))
print("\nInterpretation:")
if motion:
    print("  Drag motion WAS reported. The terminal reports and selects at the")
    print("  same time, so mouse capture does not cost native selection, and")
    print("  probe-altscreen.sh's negatives are a bug in the probe.")
elif press or release:
    print("  Presses arrived but NO drag motion. The terminal withheld the drag")
    print("  while still reporting clicks -- which is exactly the behaviour the")
    print("  pane wants, and it is not explained by the mode set alone.")
else:
    print("  No mouse reports at all. Either no gesture was made, or reporting")
    print("  was not active for it.")
print("\nRaw log: %s" % log_path)
