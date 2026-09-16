#!/usr/bin/env python3
"""Log what the TERMINAL sends a TUI, to settle what happens during a drag.

Every probe so far measured what an application *writes*. The open question is
the other direction: while you drag to select inside Copilot, does the terminal
deliver mouse reports to it, or keep the drag for itself?

    python3 notes/capture-input.py copilot

Use the program, DRAG TO SELECT SOME TEXT, then quit it normally.

Note on the implementation: `pty.spawn` is not usable here. It never sets the
child pty's window size, so a full-screen TUI renders into a phantom 80x24 (or
0x0) and shows a blank screen. This forks the pty itself, copies the real
window size onto it, and forwards SIGWINCH.
"""
import fcntl
import os
import pty
import re
import select
import signal
import struct
import sys
import termios
import time
import tty

if len(sys.argv) < 2:
    sys.exit("usage: capture-input.py <program> [args...]")

log_path = "/tmp/capture-input-%d.log" % os.getpid()
log = open(log_path, "wb")
events = []
SGR = re.compile(rb"\x1b\[<(\d+);(\d+);(\d+)([Mm])")


def set_winsize(fd):
    try:
        cols, rows = os.get_terminal_size(0)
    except OSError:
        rows, cols = 24, 80
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))


pid, master = pty.fork()
if pid == 0:
    os.execvp(sys.argv[1], sys.argv[1:])
    os._exit(1)

set_winsize(master)
signal.signal(signal.SIGWINCH, lambda *_: set_winsize(master))

stdin_is_tty = os.isatty(0)
old = termios.tcgetattr(0) if stdin_is_tty else None
if stdin_is_tty:
    tty.setraw(0)

try:
    while True:
        try:
            ready, _, _ = select.select([0, master], [], [])
        except (InterruptedError, OSError):
            continue
        if 0 in ready:
            data = os.read(0, 4096)
            if not data:
                break
            log.write(b"%.3f IN  %r\n" % (time.time(), data))
            log.flush()
            for m in SGR.finditer(data):
                events.append((int(m.group(1)), m.group(4).decode()))
            os.write(master, data)
        if master in ready:
            try:
                data = os.read(master, 4096)
            except OSError:
                break
            if not data:
                break
            os.write(1, data)
finally:
    if stdin_is_tty:
        termios.tcsetattr(0, termios.TCSAFLUSH, old)
    log.close()
    try:
        os.waitpid(pid, 0)
    except OSError:
        pass

# SGR button byte: low 2 bits select the button, 4 = Shift, 8 = Alt, 16 = Ctrl,
# 32 = motion, 64 = wheel.
press = [b for b, k in events if k == "M" and b < 32]
motion = [b for b, k in events if (b & 32) and b < 64]
release = [b for b, k in events if k == "m"]
wheel = [b for b, k in events if b >= 64]
shifted = [b for b, _ in events if b & 4]

print("\r\n=== what the terminal sent %s ===" % sys.argv[1])
print("  button presses : %d" % len(press))
print("  DRAG MOTION    : %d   <-- the decisive number" % len(motion))
print("  releases       : %d" % len(release))
print("  wheel          : %d" % len(wheel))
print("  Shift held     : %d of %d reports" % (len(shifted), len(events)))
print("\nInterpretation:")
if motion:
    print("  Drag motion WAS reported. The terminal reports and selects at the")
    print("  same time, so capture does not cost native selection, and")
    print("  probe-altscreen.sh's negatives are a bug in the probe.")
elif press or release:
    print("  Presses arrived but NO drag motion. The terminal withheld the drag")
    print("  while still reporting clicks -- exactly what the pane wants, and")
    print("  not explained by the mode set alone.")
else:
    print("  No mouse reports at all. Either no gesture was made, or reporting")
    print("  was not active for it.")
if shifted:
    print("  NOTE: %d report(s) carried Shift -- check whether the drag that" % len(shifted))
    print("  selected was actually a Shift+drag.")
print("\nRaw log: %s" % log_path)
