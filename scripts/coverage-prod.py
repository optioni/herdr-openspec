#!/usr/bin/env python3
"""COVERAGE-PROD - the production-slice coverage floor.

`cargo llvm-cov --fail-under-lines 80` is dominated by test-module lines: the total
floor does not fire until production coverage falls below ~44%. This checker reads
the JSON export `make coverage` writes alongside that run and enforces a second,
production-only floor.

A production line is one NOT inside the brace extent of any `#[cfg(test)]` item. This
is deliberately NOT "everything above the file's first line-anchored `#[cfg(test)]`"
(the `prod()` cut `READONLY-UI`, `NOBLOCK`, and `WIRED` use) - that cut misclassifies
6,281 production lines tree-wide because src/changes.rs, src/cli.rs, and src/lib.rs
each hold more than one `#[cfg(test)]` item. See
openspec/changes/gate-integrity/design.md -> Decision 1a.

Line-counting rule (stated because two exist, per Decision 1a): a line is
"instrumented" when at least one of its segments carries `hasCount`, and "covered"
when the maximum count among those segments exceeds zero. This is the `hasCount`
rule, not cargo-llvm-cov's own per-function `totals`, which yields a different
denominator for the same report.

Known limit, on the same terms NOSPAWN-GREP and WIRED record theirs: the brace
counter used to find a #[cfg(test)] item's extent does not parse Rust, only mask
comments and string/char literal content (so an unbalanced brace inside a string
cannot desync it - see mask_non_code below). The number of extents found is printed
per file specifically so a tracker defeated in some other way shows up as a
suspicious count rather than a silently shifted floor.

usage: python3 coverage-prod.py <llvm-cov-json-report> [<degraded-coverage-map>]
environment:
    PROD_MIN                overrides the floor (a percentage, e.g. "80"). Defaults
                             to the constant below, which SHALL be this checker's
                             own measured production-slice figure on the
                             unmodified tree, rounded down to a whole point - not
                             a number transcribed from a planning document.

gate-integrity task 6.4: this checker also requires every line of every
`covers` range in a degraded-coverage map to be executed - a `proof` that
resolves and a `covers` range that resolves (tests/degraded_coverage.rs's own
job) still leave open whether the degraded path actually RAN, which only a
real per-line coverage report can answer. The map path is the checker's
SECOND positional argument, not an environment variable and not a default:
the covers-range check runs only when a map is named on the command line, so
the enforcing invocation (`make coverage`, which always names
tests/degraded-coverage.toml) opts the check IN rather than every other
caller having to opt it OUT through an escape-hatch environment variable -
gate-integrity's own correction, task 6.4b, removed the environment-variable
skip flag this check used to carry, as a waiver inside the guard whose whole
thesis is that waivers are the defect. `tests/coverage_prod.rs`'s
production-floor fixtures accordingly pass no second argument at all; its
covers-range scenarios pass a small fixture map as that argument instead of
the real one.

The map is read with a small, regex-based extractor (`parse_covers_map`
below) rather than a TOML parser, on `scripts/gates/gate-mech1.py`'s own
established terms for a narrow, fully-controlled, checked-in input shape -
the file is never third-party input, and pulling in a TOML dependency
(stdlib `tomllib` needs Python 3.11, which `AGENTS.md` does not otherwise
require of this repository) to parse six fixed keys would be new machinery
for a fact this script can read directly. Known limit, recorded on
`mask_non_code`'s own terms: the extractor does not parse Rust either, so a
`covers` range is checked whether or not it is actually within the
array-of-tables structure the coverage spec assumes - it is checked against
the named map file itself, which `tests/degraded_coverage.rs` already keeps
honest for the real tests/degraded-coverage.toml.
"""
import json
import os
import re
import sys
from pathlib import Path

# Task 1.3: this SHALL be the checker's own output on the unmodified tree, rounded
# down to a whole point. See openspec/changes/gate-integrity/design.md -> Decision 1a
# and Risks ("PROD_MIN is set from the checker's own output at implementation time").
#
# Measured at implementation time (task 1.3), from this checker's own hasCount rule
# against a real `cargo llvm-cov --json` export of the unmodified tree (25 files under
# src/, 22,285 hasCount-instrumented lines total, matching design.md's own total under
# the same rule): production 96.07% (3544/3689 lines), test-module 96.43%
# (17933/18596). This differs from design.md's own 97.28% estimate (6623/6808) because
# the tree has grown since that estimate was written; per Decision 1a, the figure
# actually produced here - not the design document's - is what SHALL be recorded.
DEFAULT_PROD_MIN = 96

SRC_COMPONENT = "src"


def fail(msg: str) -> None:
    print(f"COVERAGE-PROD FAIL: {msg}", file=sys.stderr)
    sys.exit(1)


def mask_non_code(text: str) -> str:
    """Replaces every comment and the content of every string/char literal with
    spaces of equal length, preserving newlines (and therefore exact line numbers),
    so the brace counter in cfg_test_extents cannot be desynced by a brace character
    that happens to sit inside a string literal (this crate's own tests hold several
    r#"{"schemaName":...}"# JSON fixtures) or inside a comment. Adapted from
    scripts/gates/gate-mech1.py's strip_comments, which instead leaves string
    content intact; this checker additionally blanks it, because a brace inside a
    string would otherwise unbalance brace-depth counting rather than merely being
    counted as a `..` false positive.
    """

    def blank(s: str) -> str:
        return "".join(ch if ch == "\n" else " " for ch in s)

    out = []
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        if c == "r":  # raw string literal
            m = re.match(r'r(#*)"', text[i:])
            if m:
                hashes = m.group(1)
                terminator = '"' + hashes
                start = i + m.end()
                end = text.find(terminator, start)
                end = n if end == -1 else end + len(terminator)
                out.append(blank(text[i:end]))
                i = end
                continue
        if c == '"':  # string literal
            j = i + 1
            while j < n:
                if text[j] == "\\" and j + 1 < n:
                    j += 2
                    continue
                if text[j] == '"':
                    j += 1
                    break
                j += 1
            out.append(blank(text[i:j]))
            i = j
            continue
        if c == "'" and i + 1 < n:  # char literal or lifetime
            m = re.match(r"'(?:\\.|[^\\'])'", text[i:])
            if m:
                out.append(blank(m.group(0)))
                i += len(m.group(0))
                continue
            out.append(c)
            i += 1
            continue
        if text.startswith("//", i):  # line comment
            j = text.find("\n", i)
            j = n if j == -1 else j
            out.append(blank(text[i:j]))
            i = j
            continue
        if text.startswith("/*", i):  # block comment, nesting
            depth, j = 1, i + 2
            while j < n and depth:
                if text.startswith("/*", j):
                    depth += 1
                    j += 2
                elif text.startswith("*/", j):
                    depth -= 1
                    j += 2
                else:
                    j += 1
            out.append(blank(text[i:j]))
            i = j
            continue
        out.append(c)
        i += 1
    return "".join(out)


CFG_TEST_RE = re.compile(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]")


def lineno(text: str, pos: int) -> int:
    return text.count("\n", 0, pos) + 1


def cfg_test_extents(masked: str) -> list:
    """Returns one (start_line, end_line) 1-indexed inclusive range per `#[cfg(test)]`
    attribute found in `masked`, spanning from the attribute's own line through the
    end of the brace-delimited item it decorates, or through the terminating `;` for
    a bodyless item (a `#[cfg(test)] use ...;` or `#[cfg(test)] static ...;`)."""
    extents = []
    n = len(masked)
    for m in CFG_TEST_RE.finditer(masked):
        start_line = lineno(masked, m.start())
        i = m.end()
        depth = 0
        seen_brace = False
        end_pos = n - 1
        while i < n:
            c = masked[i]
            if c == "{":
                depth += 1
                seen_brace = True
            elif c == "}":
                depth -= 1
                if seen_brace and depth == 0:
                    end_pos = i
                    break
            elif c == ";" and not seen_brace and depth == 0:
                end_pos = i
                break
            i += 1
        extents.append((start_line, lineno(masked, end_pos)))
    return extents


def load_report(path: Path):
    if not path.is_file():
        fail(f"report file not found: {path}")
    try:
        raw = path.read_text()
    except OSError as e:
        fail(f"cannot read report {path}: {e}")
    try:
        return json.loads(raw)
    except json.JSONDecodeError as e:
        fail(f"cannot parse JSON report {path}: {e}")


def iter_report_files(data) -> list:
    files = []
    if not isinstance(data, dict):
        return files
    for entry in data.get("data", []) or []:
        if not isinstance(entry, dict):
            continue
        for f in entry.get("files", []) or []:
            if isinstance(f, dict):
                files.append(f)
    return files


def is_under_src(filename: str) -> bool:
    return SRC_COMPONENT in Path(filename).parts


# A small, regex-based extractor for exactly the two keys this check needs from each
# [[row]] block - see the module docstring's "gate-integrity task 6.4" note for why this
# is not a TOML parser.
ROW_BLOCK_RE = re.compile(r"\[\[row\]\](.*?)(?=\n\[\[row\]\]|\Z)", re.S)
CONDITION_RE = re.compile(r'condition\s*=\s*"((?:[^"\\]|\\.)*)"')
COVERS_ARRAY_RE = re.compile(r"covers\s*=\s*\[(.*?)\]", re.S)
COVERS_ITEM_RE = re.compile(r'"((?:[^"\\]|\\.)*)"')


def parse_covers_map(text: str) -> list:
    """Returns one (condition, [(path, first, last), ...]) pair per `[[row]]` block found
    in `text` - a row with no `covers` array, or an unparseable one, contributes an empty
    range list rather than being dropped, so the caller's own row/range count comparison
    stays meaningful."""
    rows = []
    for block in ROW_BLOCK_RE.findall(text):
        cond_m = CONDITION_RE.search(block)
        if not cond_m:
            continue
        condition = cond_m.group(1)
        entries = []
        covers_m = COVERS_ARRAY_RE.search(block)
        if covers_m:
            for item in COVERS_ITEM_RE.findall(covers_m.group(1)):
                path, sep, rng = item.rpartition(":")
                if not sep or "-" not in rng:
                    continue
                first_s, _, last_s = rng.partition("-")
                try:
                    entries.append((path, int(first_s), int(last_s)))
                except ValueError:
                    continue
        rows.append((condition, entries))
    return rows


def build_line_counts(report_data) -> dict:
    """Per-file `{line: max(count)}`, over every `hasCount` segment in every file the
    report names - not only the ones under `src/`, since a `covers` fixture in
    `tests/fixtures/coverage/` legitimately points elsewhere."""
    line_counts = {}
    for f in iter_report_files(report_data):
        filename = f.get("filename")
        if not isinstance(filename, str):
            continue
        counts = {}
        for seg in f.get("segments") or []:
            if not isinstance(seg, (list, tuple)) or len(seg) < 4:
                continue
            line, _col, count, has_count = seg[0], seg[1], seg[2], seg[3]
            if not has_count:
                continue
            prev = counts.get(line)
            if prev is None or count > prev:
                counts[line] = count
        line_counts[filename] = counts
    return line_counts


def find_counts_for_path(line_counts: dict, path: str):
    """Match a `covers` entry's `path` against the report's own filenames - exactly, or as
    a `/`-bounded suffix, since a real `cargo llvm-cov` report names files by an absolute
    path (see `classify_file`'s own resolution) while a fixture report may name one
    relatively already."""
    for filename, counts in line_counts.items():
        if filename == path or filename.endswith("/" + path):
            return counts
    return None


def check_covers_ranges(report_data, toml_path: Path) -> None:
    """gate-integrity task 6.4: every line of every `covers` range in the degraded-coverage
    map SHALL be executed. Fails naming the offending row's `condition`, per the spec, and
    cannot pass vacuously: an unreadable map, a map with no rows, an entry whose every
    `covers` array is empty, or a map holding fewer total ranges than rows are each a
    failure, not a report of 100% over nothing."""
    if not toml_path.is_file():
        fail(f"degraded-coverage map not found: {toml_path}")
    text = toml_path.read_text()

    total_row_blocks = text.count("[[row]]")
    if total_row_blocks == 0:
        fail(f"{toml_path} names no [[row]] entries - refusing to report every range covered")

    rows = parse_covers_map(text)
    total_ranges = sum(len(entries) for _, entries in rows)
    if total_ranges == 0:
        fail(
            f"{toml_path}: every row's \"covers\" array is empty - refusing to report "
            "every range covered"
        )
    if total_ranges < total_row_blocks:
        fail(
            f"{toml_path} holds {total_row_blocks} row(s) but only {total_ranges} "
            "covers range(s) in total - dropping a range is a failure, not a silent "
            "reduction in what is proved"
        )

    line_counts = build_line_counts(report_data)
    failures = []
    for condition, entries in rows:
        for path, first, last in entries:
            counts = find_counts_for_path(line_counts, path)
            if counts is None:
                failures.append(f"{condition!r}: {path}:{first}-{last} - report names no such file")
                continue
            instrumented = [ln for ln in range(first, last + 1) if ln in counts]
            if not instrumented:
                failures.append(
                    f"{condition!r}: {path}:{first}-{last} - report instruments no line "
                    "in this range"
                )
                continue
            cold = [ln for ln in instrumented if counts[ln] == 0]
            if cold:
                failures.append(
                    f"{condition!r}: {path}:{first}-{last} has uncovered line(s) {cold}"
                )
    if failures:
        fail("degraded-coverage covers range(s) uncovered:\n  " + "\n  ".join(failures))

    print(
        f"COVERAGE-PROD DEGRADED-COVERS OK: {total_row_blocks} row(s), {total_ranges} "
        f"range(s) from {toml_path}, all covered"
    )


def classify_file(filename: str, segments: list) -> dict:
    src_path = Path(filename)
    if not src_path.is_file():
        fail(
            f"cannot locate source file for report entry {filename!r} "
            f"(expected it at that path, relative to the working directory)"
        )
    text = src_path.read_text()
    masked = mask_non_code(text)
    extents = cfg_test_extents(masked)

    test_lines = set()
    for start, end in extents:
        test_lines.update(range(start, end + 1))

    line_count = {}
    for seg in segments:
        if not isinstance(seg, (list, tuple)) or len(seg) < 4:
            continue
        line, _col, count, has_count = seg[0], seg[1], seg[2], seg[3]
        if not has_count:
            continue
        prev = line_count.get(line)
        if prev is None or count > prev:
            line_count[line] = count

    prod_instr = prod_cov = test_instr = test_cov = 0
    for line, count in line_count.items():
        if line in test_lines:
            test_instr += 1
            if count > 0:
                test_cov += 1
        else:
            prod_instr += 1
            if count > 0:
                prod_cov += 1

    return {
        "extents": len(extents),
        "prod_instr": prod_instr,
        "prod_cov": prod_cov,
        "test_instr": test_instr,
        "test_cov": test_cov,
    }


def main(argv: list) -> int:
    if len(argv) < 2:
        fail("usage: python3 coverage-prod.py <llvm-cov-json-report> [<degraded-coverage-map>]")
    report_path = Path(argv[1])
    covers_map_path = Path(argv[2]) if len(argv) > 2 else None

    floor = DEFAULT_PROD_MIN
    override = os.environ.get("PROD_MIN")
    if override:
        try:
            floor = float(override)
        except ValueError:
            fail(f"PROD_MIN={override!r} is not a number")

    data = load_report(report_path)

    if covers_map_path is not None:
        check_covers_ranges(data, covers_map_path)

    all_files = iter_report_files(data)
    src_files = [
        f
        for f in all_files
        if isinstance(f.get("filename"), str) and is_under_src(f["filename"])
    ]

    if not src_files:
        fail(
            f"report names no file under {SRC_COMPONENT!r}/ "
            f"({len(all_files)} file(s) total) - refusing to report 100% of nothing"
        )

    totals = {"prod_instr": 0, "prod_cov": 0, "test_instr": 0, "test_cov": 0}
    per_file = []
    for f in src_files:
        filename = f["filename"]
        result = classify_file(filename, f.get("segments") or [])
        per_file.append((filename, result))
        for k in totals:
            totals[k] += result[k]
        print(
            f"COVERAGE-PROD FILE {filename} extents={result['extents']} "
            f"prod_instrumented={result['prod_instr']} prod_covered={result['prod_cov']} "
            f"test_instrumented={result['test_instr']} test_covered={result['test_cov']}"
        )

    if totals["prod_instr"] == 0:
        fail(
            "report names 0 production-instrumented lines under "
            f"{SRC_COMPONENT}/ - refusing to report 100% of nothing"
        )

    prod_pct = 100.0 * totals["prod_cov"] / totals["prod_instr"]
    test_pct = (
        100.0 * totals["test_cov"] / totals["test_instr"] if totals["test_instr"] else 0.0
    )

    if prod_pct < floor:
        worst = sorted(
            per_file,
            key=lambda kv: kv[1]["prod_instr"] - kv[1]["prod_cov"],
            reverse=True,
        )
        worst = [
            f"{name} ({r['prod_instr'] - r['prod_cov']})"
            for name, r in worst
            if r["prod_instr"] - r["prod_cov"] > 0
        ][:5]
        fail(
            f"production coverage {prod_pct:.2f}% ({totals['prod_cov']}/{totals['prod_instr']}) "
            f"< floor {floor}% (line hasCount/max-count rule); "
            f"most uncovered production lines: {', '.join(worst) or 'n/a'}"
        )

    print(
        f"COVERAGE-PROD OK: production {prod_pct:.2f}% "
        f"({totals['prod_cov']}/{totals['prod_instr']}) >= floor {floor}%; "
        f"test-module {test_pct:.2f}% ({totals['test_cov']}/{totals['test_instr']}); "
        f"{len(src_files)} file(s) under {SRC_COMPONENT}/ (line hasCount/max-count rule)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
