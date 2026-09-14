# HEAD output baseline

Captured at HEAD (task 0.5) before any edit of `tasks-emphasis`.
Every string below is a `Line::text()` value, Rust-debug-quoted.

## `ui::tasks::items`

### three-segment fixture: `- [ ] 1.1 RED: write the failing test` / `- [ ] Commit: the parser` — width 78

```
"[ ] 1.1 RED: write the failing test"
"[ ] Commit: the parser"
```

### three-segment fixture: `- [ ] 1.1 RED: write the failing test` / `- [ ] Commit: the parser` — width 58

```
"[ ] 1.1 RED: write the failing test"
"[ ] Commit: the parser"
```

### three-segment fixture: `- [ ] 1.1 RED: write the failing test` / `- [ ] Commit: the parser` — width 16

```
"[ ] 1.1 RED:"
"    write the"
"    failing test"
"[ ] Commit: the"
"    parser"
```

### three-segment fixture: `- [ ] 1.1 RED: write the failing test` / `- [ ] Commit: the parser` — width 14

```
"[ ] 1.1 RED:"
"    write the"
"    failing"
"    test"
"[ ] Commit:"
"    the parser"
```

### three-segment fixture: `- [ ] 1.1 RED: write the failing test` / `- [ ] Commit: the parser` — width 12

```
"[ ] 1.1 RED:"
"    write"
"    the"
"    failing"
"    test"
"[ ] Commit:"
"    the"
"    parser"
```

### checked fixture: `- [x] 1.1 VERIFY: make check is green` — width 78

```
"[✓] 1.1 VERIFY: make check is green"
```

### checked fixture: `- [x] 1.1 VERIFY: make check is green` — width 58

```
"[✓] 1.1 VERIFY: make check is green"
```

### checked fixture: `- [x] 1.1 VERIFY: make check is green` — width 16

```
"[✓] 1.1 VERIFY:"
"    make check"
"    is green"
```

### checked fixture: `- [x] 1.1 VERIFY: make check is green` — width 14

```
"[✓] 1.1"
"    VERIFY:"
"    make check"
"    is green"
```

### checked fixture: `- [x] 1.1 VERIFY: make check is green` — width 12

```
"[✓] 1.1"
"    VERIFY:"
"    make"
"    check is"
"    green"
```

### unchecked twin: `- [ ] 1.1 VERIFY: make check is green` — width 78

```
"[ ] 1.1 VERIFY: make check is green"
```

### unchecked twin: `- [ ] 1.1 VERIFY: make check is green` — width 58

```
"[ ] 1.1 VERIFY: make check is green"
```

### unchecked twin: `- [ ] 1.1 VERIFY: make check is green` — width 16

```
"[ ] 1.1 VERIFY:"
"    make check"
"    is green"
```

### unchecked twin: `- [ ] 1.1 VERIFY: make check is green` — width 14

```
"[ ] 1.1"
"    VERIFY:"
"    make check"
"    is green"
```

### unchecked twin: `- [ ] 1.1 VERIFY: make check is green` — width 12

```
"[ ] 1.1"
"    VERIFY:"
"    make"
"    check is"
"    green"
```

### wrapped fixture: `1.1 GREEN:` + twenty eight-character words — width 78

```
"[ ] 1.1 GREEN: abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh"
"    abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh"
"    abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh"
```

### wrapped fixture: `1.1 GREEN:` + twenty eight-character words — width 58

```
"[ ] 1.1 GREEN: abcdefgh abcdefgh abcdefgh abcdefgh"
"    abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh"
"    abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh"
"    abcdefgh abcdefgh abcdefgh abcdefgh"
```

### wrapped fixture: `1.1 GREEN:` + twenty eight-character words — width 16

```
"[ ] 1.1 GREEN:"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
```

### wrapped fixture: `1.1 GREEN:` + twenty eight-character words — width 14

```
"[ ] 1.1 GREEN:"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
```

### wrapped fixture: `1.1 GREEN:` + twenty eight-character words — width 12

```
"[ ] 1.1"
"    GREEN:"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
"    abcdefgh"
```

### wrap-split fixture: `- [ ] 1.1 CHARACTERIZE: record the baseline` — width 78

```
"[ ] 1.1 CHARACTERIZE: record the baseline"
```

### wrap-split fixture: `- [ ] 1.1 CHARACTERIZE: record the baseline` — width 58

```
"[ ] 1.1 CHARACTERIZE: record the baseline"
```

### wrap-split fixture: `- [ ] 1.1 CHARACTERIZE: record the baseline` — width 16

```
"[ ] 1.1"
"    CHARACTERIZE"
"    : record the"
"    baseline"
```

### wrap-split fixture: `- [ ] 1.1 CHARACTERIZE: record the baseline` — width 14

```
"[ ] 1.1"
"    CHARACTERI"
"    ZE: record"
"    the"
"    baseline"
```

### wrap-split fixture: `- [ ] 1.1 CHARACTERIZE: record the baseline` — width 12

```
"[ ] 1.1"
"    CHARACTE"
"    RIZE:"
"    record"
"    the"
"    baseline"
```

## `ui::tasks::lines`

### one group — width 78

```
"██████████████████████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ [1/2] 50%"
""
"## 1. Setup"
"[✓] 1.1 first"
"[ ] 1.2 second"
```

### one group — width 58

```
"████████████████████████░░░░░░░░░░░░░░░░░░░░░░░░ [1/2] 50%"
""
"## 1. Setup"
"[✓] 1.1 first"
"[ ] 1.2 second"
```

### two groups — width 78

```
"██████████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ [1/3] 33%"
""
"## 1. Setup"
"[✓] 1.1 first"
"[ ] 1.2 second"
""
"## 2. Build"
"[ ] 2.1 third"
```

### two groups — width 58

```
"████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ [1/3] 33%"
""
"## 1. Setup"
"[✓] 1.1 first"
"[ ] 1.2 second"
""
"## 2. Build"
"[ ] 2.1 third"
```

## `ui::markdown::lines`

### group-3 document — width 78

```
"# Heading"
""
"A paragraph of prose."
""
"• alpha"
"• bravo"
""
"cargo test"
""
""
"│ quoted line"
""
"A link here."
""
"A struck run."
""
"│ a │ b │"
"├───┼───┤"
"│ 1 │ 2 │"
""
"[✓] done item"
"[ ] open item"
""
"VERIFY: this is prose, not a task"
```

Every segment's face, width 78:

```
"# " Face { heading: Some(1), strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"Heading" Face { heading: Some(1), strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"A paragraph of prose." Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"• " Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"alpha" Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"• " Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"bravo" Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"cargo test" Face { heading: None, strong: false, emphasis: false, code: true, link: false, quoted: false, strikethrough: false }
"│ " Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: true, strikethrough: false }
"quoted line" Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: true, strikethrough: false }
"A " Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"link" Face { heading: None, strong: false, emphasis: false, code: false, link: true, quoted: false, strikethrough: false }
" here." Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"A " Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"struck" Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: true }
" run." Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"│ " Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"a" Face { heading: None, strong: true, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
" │ " Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"b" Face { heading: None, strong: true, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
" │" Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"├───┼───┤" Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"│ 1 │ 2 │" Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"[✓] " Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"done item" Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"[ ] " Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"open item" Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"VERIFY: this is prose, not a task" Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
```

### group-3 document — width 58

```
"# Heading"
""
"A paragraph of prose."
""
"• alpha"
"• bravo"
""
"cargo test"
""
""
"│ quoted line"
""
"A link here."
""
"A struck run."
""
"│ a │ b │"
"├───┼───┤"
"│ 1 │ 2 │"
""
"[✓] done item"
"[ ] open item"
""
"VERIFY: this is prose, not a task"
```

Every segment's face, width 58:

```
"# " Face { heading: Some(1), strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"Heading" Face { heading: Some(1), strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"A paragraph of prose." Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"• " Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"alpha" Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"• " Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"bravo" Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"cargo test" Face { heading: None, strong: false, emphasis: false, code: true, link: false, quoted: false, strikethrough: false }
"│ " Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: true, strikethrough: false }
"quoted line" Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: true, strikethrough: false }
"A " Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"link" Face { heading: None, strong: false, emphasis: false, code: false, link: true, quoted: false, strikethrough: false }
" here." Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"A " Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"struck" Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: true }
" run." Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"│ " Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"a" Face { heading: None, strong: true, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
" │ " Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"b" Face { heading: None, strong: true, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
" │" Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"├───┼───┤" Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"│ 1 │ 2 │" Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"[✓] " Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"done item" Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"[ ] " Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"open item" Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
"VERIFY: this is prose, not a task" Face { heading: None, strong: false, emphasis: false, code: false, link: false, quoted: false, strikethrough: false }
```
