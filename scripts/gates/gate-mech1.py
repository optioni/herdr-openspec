#!/usr/bin/env python3
"""GATE-MECH1 - mechanism 1 of change-model's two-producer gate.

Half A: no `Default` is reachable for Change, ChangeSet, ArtifactRef, or Origin,
        anywhere under the source tree - not just in changes.rs, since
        `impl Default for Change` is legal in any file of the crate.
Half B: no `..` functional update and no `..` rest pattern in changes.rs.

    usage: python3 GATE-MECH1.py [SRC_DIR]      (default: src)
    exit 0 only when BOTH halves pass.
"""
import re
import sys
from pathlib import Path

TYPES = ("Change", "ChangeSet", "ArtifactRef", "Origin")


def strip_comments(text: str) -> str:
    """Replace every comment with an equal number of spaces/newlines, so byte
    offsets - and therefore reported line numbers - stay exact. String and char
    literals are left intact; a comment marker inside one is not a comment.

    Handles Rust raw string literals (`r"..."`, `r#"..."#`, `r##"..."##`, ...)
    as a distinct case from a plain `"..."` string: a naive scan for the next
    unescaped `"` desyncs on a raw string's own embedded quotes (this crate's
    tests hold several `r#"{"schemaName":...}"#` JSON fixtures), and once
    desynced treats every following genuine `//` comment as "inside a string"
    and leaves it unstripped — found via task 8.5's GATE-MECH1 run, which
    reported a `..` inside an English-prose comment ("(alpha < design <
    proposal, ...)") because the raw-string fixtures earlier in the same file
    had thrown the scan off; the group's own `..` **usages** (`CliError`/
    `LoadError` variants, unrelated to the four gated types but still forbidden
    by the file-wide half B rule) were separately named-out, not exempted."""
    out = []
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        if c == "r":                                    # raw string literal
            m = re.match(r'r(#*)"', text[i:])
            if m:
                hashes = m.group(1)
                terminator = '"' + hashes
                start = i + m.end()
                end = text.find(terminator, start)
                end = n if end == -1 else end + len(terminator)
                out.append(text[i:end]); i = end; continue
        if c == '"':                                   # string literal
            out.append(c); i += 1
            while i < n:
                if text[i] == "\\" and i + 1 < n:
                    out.append(text[i:i + 2]); i += 2; continue
                out.append(text[i])
                if text[i] == '"':
                    i += 1; break
                i += 1
            continue
        if c == "'" and i + 1 < n:                     # char literal or lifetime
            m = re.match(r"'(?:\\.|[^\\'])'", text[i:])
            if m:
                out.append(m.group(0)); i += len(m.group(0)); continue
            out.append(c); i += 1; continue
        if text.startswith("//", i):                   # line comment
            j = text.find("\n", i)
            j = n if j == -1 else j
            out.append(" " * (j - i)); i = j; continue
        if text.startswith("/*", i):                   # block comment, nesting
            depth, j = 1, i + 2
            while j < n and depth:
                if text.startswith("/*", j):
                    depth += 1; j += 2
                elif text.startswith("*/", j):
                    depth -= 1; j += 2
                else:
                    j += 1
            out.append("".join(ch if ch == "\n" else " " for ch in text[i:j]))
            i = j; continue
        out.append(c); i += 1
    return "".join(out)


def lineno(text: str, pos: int) -> int:
    return text.count("\n", 0, pos) + 1


def fail(msg: str) -> None:
    print(f"GATE-MECH1 FAIL: {msg}", file=sys.stderr)
    sys.exit(1)


src = Path(sys.argv[1] if len(sys.argv) > 1 else "src")
if not src.is_dir():
    fail(f"no such directory: {src}")
files = sorted(src.rglob("*.rs"))
# Vacuity guard A - the searched set is the crate's real module set, not an
# empty list. 8 is today's count; a module deliberately removed is a deliberate
# edit here.
if len(files) < 8:
    fail(f"searched only {len(files)} .rs files under {src} (expected >= 8)")

changes = src / "changes.rs"
if not changes.is_file():
    fail(f"{changes} missing - nothing to check")
changes_body = strip_comments(changes.read_text())

# Vacuity guard B - all four types must be declared where we expect them, or a
# search that found no Default found nothing because it looked in the wrong place.
for decl in ("pub struct Change {", "pub struct ChangeSet {",
             "pub struct ArtifactRef {", "pub enum Origin {"):
    if decl not in changes_body:
        fail(f"{changes} does not declare '{decl}'")

# --- Half A: no Default for the four types, anywhere under src/ --------------
impl_re = re.compile(
    r"impl\b[^;{]*?\b(?:std::default::|core::default::)?Default\s+for\s+"
    r"(?:crate::)?(?:changes::)?(" + "|".join(TYPES) + r")\b")
decl_re = re.compile(r"(?:pub\s+)?(?:struct|enum)\s+(" + "|".join(TYPES) + r")\b")

hits_a = []
for f in files:
    body = strip_comments(f.read_text())
    for m in impl_re.finditer(body):
        hits_a.append(f"{f}:{lineno(body, m.start())}: {m.group(0).strip()}")
    for m in decl_re.finditer(body):
        # Walk back over the attribute block immediately above the declaration,
        # so a multi-line `#[derive(\n ... Default,\n)]` is caught too.
        head = body[:m.start()]
        block = head[head.rfind("\n\n") + 1:] if "\n\n" in head else head
        for d in re.finditer(r"#\[derive\(([^)]*)\)\]", block, re.S):
            if re.search(r"\bDefault\b", d.group(1)):
                hits_a.append(
                    f"{f}:{lineno(body, m.start())}: derive(Default) on {m.group(1)}")
if hits_a:
    print("GATE-MECH1 FAIL (half A): Default reachable for a Change type:",
          file=sys.stderr)
    for h in hits_a:
        print("  " + h, file=sys.stderr)
    sys.exit(1)

# --- Half B: no `..` functional update and no `..` rest pattern in changes.rs -
# A `..` whose nearest preceding non-whitespace character is `,` or `{` is a
# functional update or a rest pattern; those are the only two positions either
# can occupy. `segment[..star]` is preceded by `[` and is therefore not a hit -
# that slice index is this half's own discriminating positive control.
ctor_re = re.compile(r"(?:Change|ChangeSet|ArtifactRef|Origin::Archived)\s*\{")
n_ctor = len(ctor_re.findall(changes_body))
if n_ctor < 3:
    fail(f"found only {n_ctor} struct constructions in {changes} (expected >= 3)")

hits_b = [f"{changes}:{lineno(changes_body, m.start())}: "
          f"{changes_body[m.start():m.start() + 40].strip()!r}"
          for m in re.finditer(r"[,{]\s*\.\.", changes_body)]
if hits_b:
    print("GATE-MECH1 FAIL (half B): rest pattern or functional update:",
          file=sys.stderr)
    for h in hits_b:
        print("  " + h, file=sys.stderr)
    sys.exit(1)

print(f"GATE-MECH1 OK (half A): no Default for {', '.join(TYPES)} "
      f"in {len(files)} files under {src}")
print(f"GATE-MECH1 OK (half B): {n_ctor} constructions in {changes}, "
      f"no rest pattern and no functional update")
