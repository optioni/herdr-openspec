#!/usr/bin/env python3
"""Measure the tasks.md corpus the way `tasks::parse` will after task-item-bodies.

Applies the change's own Decision 2 continuation rule: a line continues the
preceding item when blank or indented strictly past that item's `indent`; the
body ends at the first heading, the first checkbox line at any indent, or the
first non-blank line at or below that indent; trailing blanks are stripped.

Usage: python3 scripts/measure-tasks-corpus.py [--exclude CHANGE]
"""
import glob, re, sys

exclude = None
if "--exclude" in sys.argv:
    exclude = sys.argv[sys.argv.index("--exclude") + 1]

files = sorted(glob.glob("openspec/changes/*/tasks.md") +
               glob.glob("openspec/changes/archive/*/tasks.md"))
if exclude:
    files = [f for f in files if f"/{exclude}/" not in f]

HEAD = re.compile(r'^#{1,6}( |$)')
TASK = re.compile(r'^[ \t ]*[-*][ \t ]*\[[ xX\t]\]')
NUM  = re.compile(r'^[0-9.]+[a-z]? ')
RUN  = re.compile(r'^[A-Z]{2,}')

kept = dropped = 0
items = truncated = longest = 0
body_lines = 0
numw = {}
plain_labels = emph_labels = 0
fences = ind_fences = grp_fences = 0
cb_in_fence = 0
col0_fences = col0_with_blank = 0
pre_files = pre_lines = 0

for f in files:
    lines = [l.rstrip("\r") for l in open(f, encoding="utf-8").read().split("\n")]

    # preamble: non-blank lines above the first heading or item
    for l in lines:
        if HEAD.match(l) or TASK.match(l):
            break
        if l.strip():
            pre_lines += 1
    else:
        pass
    if any(l.strip() for l in
           lines[:next((i for i, l in enumerate(lines)
                        if HEAD.match(l) or TASK.match(l)), len(lines))]):
        pre_files += 1

    # fences, and column-zero fences containing a blank line
    i = 0
    cur_indent = None
    while i < len(lines):
        l = lines[i]; s = l.strip()
        if HEAD.match(l):
            cur_indent = None
        elif TASK.match(l):
            cur_indent = len(l) - len(l.lstrip())
        if s.startswith("```") or s.startswith("~~~"):
            fences += 1
            ind = len(l) - len(l.lstrip())
            if cur_indent is not None and ind > cur_indent:
                ind_fences += 1
            else:
                grp_fences += 1
            mark = s[:3]
            j, blank = i + 1, False
            while j < len(lines) and not lines[j].strip().startswith(mark):
                if not lines[j].strip():
                    blank = True
                if TASK.match(lines[j]):
                    cb_in_fence += 1
                j += 1
            if ind == 0:
                col0_fences += 1
                if blank:
                    col0_with_blank += 1
            i = j + 1
            continue
        i += 1

    # kept / dropped / bodies, under Decision 2
    open_indent = None
    pending = []
    def close():
        global truncated, longest, body_lines
        while pending and not pending[-1].strip():
            pending.pop()
        if pending:
            truncated += 1
            body_lines += len(pending)
            globals()['longest'] = max(globals()['longest'], len(pending))
        pending.clear()

    for l in lines:
        if HEAD.match(l):
            close(); open_indent = None; kept += 1
        elif TASK.match(l):
            close()
            open_indent = len(l) - len(l.lstrip())
            kept += 1
            items += 1
            text = TASK.sub("", l).strip()
            m = NUM.match(text)
            if m:
                numw[len(m.group(0))] = numw.get(len(m.group(0)), 0) + 1
                rest = text[len(m.group(0)):]
            else:
                numw[0] = numw.get(0, 0) + 1
                rest = text
            if RUN.match(rest) and ':' in rest:
                plain_labels += 1
            elif rest[:2] in ('**', '__') or rest[:1] in ('*', '_', '`'):
                inner = rest.lstrip('*_`')
                if RUN.match(inner) and ':' in inner:
                    emph_labels += 1
        else:
            blank = not l.strip()
            ind = len(l) - len(l.lstrip())
            if not blank:
                dropped += 1
            if open_indent is not None and (blank or ind > open_indent):
                pending.append(l)
            else:
                close(); open_indent = None
    close()

print(f"files                     {len(files)}")
print(f"kept (headings+items)     {kept}")
print(f"dropped non-blank         {dropped}")
print(f"items                     {items}")
print(f"items carrying a body     {truncated}  ({100*truncated/items:.1f}%)")
print(f"body lines total          {body_lines}  ({body_lines/items:.2f} per item)")
print(f"longest body              {longest}")
print(f"task-number widths        {dict(sorted(numw.items()))}")
print(f"plain labels              {plain_labels}")
print(f"emphasised labels         {emph_labels}")
print(f"fences (all)              {fences}   indented {ind_fences}   group-level {grp_fences}")
print(f"column-zero fences        {col0_fences}   containing a blank line {col0_with_blank}")
print(f"checkbox inside a fence   {cb_in_fence}")
print(f"files with a preamble     {pre_files}   preamble non-blank lines {pre_lines}")
