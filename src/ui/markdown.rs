//! Markdown source to plain-data lines of faced segments, parameterised by
//! an interior width. A pure total transformation: no filesystem, process,
//! environment, network, or standard-I/O API, and it panics for no `&str`
//! and no `u16`. Styling a segment for the view is not this module's job —
//! see `openspec/changes/markdown-viewer/specs/markdown-render/spec.md`.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use crate::specs::{Clause, clause_of};
use crate::ui::layout::{columns, truncate_columns};

/// The glyphs this module emits that are not drawn from the source
/// document. Each measures exactly one display column under
/// `layout::columns`, which `markdown-render`'s own scenario asserts — the
/// single property every line grammar built on them rests on. Six of the
/// seven are East Asian **Ambiguous** and a CJK-locale terminal paints them
/// at two columns; `SPEC.md` records that as an accepted, uncompensated
/// exposure rather than guessing at a correction.
const BULLET: &str = "• ";
const QUOTE: &str = "│ ";
const RULE: &str = "─";
const TABLE_SEP: &str = "│";
const TABLE_RULE_LEFT: &str = "├";
const TABLE_RULE_MID: &str = "┼";
const TABLE_RULE_RIGHT: &str = "┤";
const CHECKED: &str = "[✓] ";
const UNCHECKED: &str = "[ ] ";

/// A segment's styling. A struct of flags, not an enum: markdown nests
/// (`[**bold link**](x)` is bold *and* a link), and an enum would force an
/// arbitrary precedence rule. `heading` is `Option<u8>` because the level
/// is real information the marker carries, not a flag. The only type in
/// this module that derives `Default`: a `Face` is a value with a
/// meaningful zero (unstyled), not a state type `NODEFAULT-UI` gates.
///
/// `muted` and `label` are `tasks-emphasis`' two additions, joining
/// `strikethrough`, which `markdown-legibility` added on the same terms.
/// [`lines`] returns `muted: false` on every segment it emits, for every
/// source and every width — that flag stays the checklist path's alone. They
/// exist because `Face` is the crate's one carrier of "what this run of text
/// is", and `tasks-checklist` needs to say two things about a run that no
/// markdown construct says — that a whole row is finished, and that a
/// leading token is a lifecycle label. Putting them here rather than
/// inventing a second segment type is what keeps `ui::detail::content_lines`
/// returning one line type whether its body came from the markdown path or
/// the checklist path.
///
/// `spec-emphasis` widens `label` to a second producer: a `Strong` run that
/// opens a list item and that `crate::specs::clause_of` recognises as a
/// scenario clause keyword — `- **WHEN**`, `- **THEN**`, `- **AND**`, and the
/// wider testing vocabulary the same table carries — is faced with the
/// clause's lifecycle role, on the same field a task item's leading label
/// already used. Every other segment this module emits still carries
/// `label: None`.
///
/// `label`'s type is `crate::tasks::LabelRole` — a plain enum from the parsing
/// side of the crate, reaching no I/O API and no drawing type — so it widens
/// neither confinement this module carries: not the parser's, and not the
/// "names no drawing type" rule stated above. (Both are enforced by a grep over
/// this whole file, prose included, so the sentence is worded around the names
/// they search for; that is the known limit those checks already carry, and
/// rewording is its stated repair.) This module names the type and calls no
/// function of `crate::tasks` — the classification a clause keyword needs is
/// reached through `crate::specs::clause_of` instead, which itself calls
/// `crate::tasks::role_of` so the two vocabularies never drift apart
/// (design.md -> Decision 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Face {
    pub heading: Option<u8>,
    pub strong: bool,
    pub emphasis: bool,
    pub code: bool,
    pub link: bool,
    pub quoted: bool,
    pub strikethrough: bool,
    /// De-emphasised whole: a task item the reader has finished with.
    pub muted: bool,
    /// A task's leading lifecycle label, as `crate::tasks::label_of` classifies
    /// it.
    pub label: Option<crate::tasks::LabelRole>,
    /// A delta badge's marker, as `ui::detail` sets it on a requirement
    /// section's badge segment. `spec-emphasis`' one new field, on the same
    /// terms as `muted` and `label`: no markdown construct is a delta
    /// operation, so [`lines`] leaves this `None` on every segment it
    /// returns.
    pub delta: Option<crate::specs::DeltaOp>,
}

impl Face {
    /// The all-`false`, `heading: None`, `label: None`, `delta: None` value.
    pub fn plain() -> Face {
        Face::default()
    }
}

/// One faced run of text. Never padded: a region draws segments left to
/// right and leaves the rest of its row untouched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub text: String,
    pub face: Face,
}

/// One rendered row, at most `width` characters wide (counted in `char`s).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub segments: Vec<Segment>,
}

impl Line {
    /// The segments' text concatenated in order — what a width assertion
    /// reads.
    pub fn text(&self) -> String {
        self.segments.iter().map(|s| s.text.as_str()).collect()
    }

    fn blank() -> Line {
        Line {
            segments: Vec::new(),
        }
    }
}

/// A block's category, for the blank-line separation rule: two adjacent
/// list items of the same list are not separated by a blank line, but any
/// other adjacent pair is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    Item,
    Other,
}

/// One inline run inside a block, before wrapping.
#[derive(Debug, Clone)]
struct Run {
    text: String,
    face: Face,
}

/// A table column's alignment, read from the delimiter row's own markers.
/// `pulldown_cmark::Alignment::None` — an unmarked column — is `Left`,
/// which is what "a column the delimiter row leaves unmarked SHALL be
/// padded on the right" means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Align {
    Left,
    Center,
    Right,
}

/// One table cell's inline runs, before wrapping — the same `Run` every
/// other block accumulates, so a cell's content wraps and faces by exactly
/// the rules a paragraph's does.
type Cell = Vec<Run>;

/// One table row. `header` marks the row `Tag::TableHead` opened: the row
/// whose cells carry `strong`, and the row the delimiter line follows.
#[derive(Debug, Clone)]
struct Row {
    cells: Vec<Cell>,
    header: bool,
}

/// A parsed table. `alignments` comes from the delimiter row and is what
/// **declares** the column count `n`; the rows are in source order, header
/// first. Width-independent, like every other `Block` field: the column
/// allocation happens in [`emit_table`], at a width.
#[derive(Debug, Clone)]
struct Table {
    alignments: Vec<Align>,
    rows: Vec<Row>,
}

/// What a [`Block`] is. An enum rather than a pair of mutually exclusive
/// fields, so a block that is both a rule and a table is unrepresentable
/// (design.md -> Decision 2).
#[derive(Debug, Clone)]
enum BlockKind {
    /// Ordinary content: `groups`, laid out at the block's own width.
    Flow,
    /// A thematic break: a single line of `width` dashes, ignoring every
    /// other field.
    Rule,
    /// A GFM pipe table, laid out by [`emit_table`] from its own cells;
    /// `groups` stays empty.
    Table(Table),
}

/// One block-level unit to lay out. `groups` holds one `Vec<Run>` per
/// rendered "hard line" — a hard-broken run of prose, or one verbatim
/// source line of a code or HTML block — and every group always starts a
/// fresh output row, which is what the hard-break and verbatim-line rules
/// require. A *soft* break is not a group boundary: it folds into a single
/// space so the whole run reflows at the region width.
#[derive(Debug, Clone)]
struct Block {
    kind: BlockKind,
    hard_split: bool,
    first_prefix: String,
    cont_prefix: String,
    prefix_face: Face,
    category: Category,
    groups: Vec<Vec<Run>>,
}

/// One level of list nesting: whether it numbers, the next number to
/// assign (from the list's own start value, not necessarily 1), and the
/// nesting depth for the two-columns-per-level indent.
struct ListFrame {
    ordered: bool,
    next: u64,
    depth: usize,
}

/// [`QUOTE`] repeated once per block-quote nesting level. Empty outside a
/// quote.
fn quote_prefix(depth: usize) -> String {
    QUOTE.repeat(depth)
}

/// The fold's mutable state, one field per piece of context a nested
/// construct needs. Kept as a struct rather than loose locals so the
/// Start/End dispatch reads as method calls rather than a wall of
/// parameters.
struct Folder {
    blocks: Vec<Block>,

    // The block currently accumulating content. Reset to the ambient
    // (unprefixed, unfaced) configuration whenever a Paragraph or Heading
    // closes, so stray text between recognised constructs still lands
    // somewhere rather than being dropped.
    first_prefix: String,
    cont_prefix: String,
    prefix_face: Face,
    category: Category,
    hard_split: bool,
    groups: Vec<Vec<Run>>,
    group: Vec<Run>,

    // The inline face stack for `Emphasis`/`Strong`/`Link`, seeded with
    // the current block's base face whenever it changes.
    faces: Vec<Face>,

    // `Some` while inside `Start(Image)..End(Image)`: alt text is
    // accumulated here rather than as ordinary runs, since an image is
    // rendered as a single faced run of its alt text.
    image_alt: Option<String>,
    image_face: Face,

    // Container context: how many block quotes currently wrap the
    // accumulating block, and the stack of currently-open lists (innermost
    // last), each carrying its own numbering and nesting depth.
    quote_depth: usize,
    list_stack: Vec<ListFrame>,

    // The currently-open item's nesting indent, kept because
    // `Event::TaskListMarker` arrives *after* `start_item` has already
    // built both prefixes from the bullet marker and has to rebuild them
    // from the checkbox marker instead.
    item_indent: String,

    // `Some` between `Start(Table)` and `End(Table)`: the rows accumulated
    // so far, the current row's cells, and whether that row is the header.
    // The current *cell*'s runs accumulate in `group`, exactly as a
    // paragraph's do, which is what lets a cell carry inline faces without
    // a second accumulation path.
    table: Option<TableBuilder>,

    // `spec-emphasis`: the scenario's remembered lifecycle position, so an
    // `AND` clause carries the role of the `WHEN` or `THEN` above it. Reset
    // to `None` at every heading and nowhere else — a paragraph, a blank
    // line, or a nested list never crosses a scenario boundary.
    clause_position: Option<crate::tasks::LabelRole>,

    // One entry per currently-open `Strong` span, innermost last: `Some(i)`
    // when that span opened with `group` at length `i` — a candidate
    // scenario-clause keyword, resolved once the span's whole text is known
    // at `end_strong` — and `None` for a `Strong` anywhere else, which
    // `end_strong` leaves untouched.
    strong_starts: Vec<Option<usize>>,
}

/// A table under construction. Separate from [`Table`] because a fold needs
/// the two cursors — the current row and whether it is the header row —
/// that the finished value has no use for.
struct TableBuilder {
    alignments: Vec<Align>,
    rows: Vec<Row>,
    cells: Vec<Cell>,
    header: bool,
}

impl Folder {
    fn new() -> Folder {
        Folder {
            blocks: Vec::new(),
            first_prefix: String::new(),
            cont_prefix: String::new(),
            prefix_face: Face::plain(),
            category: Category::Other,
            hard_split: false,
            groups: Vec::new(),
            group: Vec::new(),
            faces: vec![Face::plain()],
            image_alt: None,
            image_face: Face::plain(),
            quote_depth: 0,
            list_stack: Vec::new(),
            item_indent: String::new(),
            table: None,
            clause_position: None,
            strong_starts: Vec::new(),
        }
    }

    /// The base face every construction site in this module seeds its
    /// inline face stack with: unstyled, except `quoted` when currently
    /// nested inside one or more block quotes.
    fn quoted_base(&self) -> Face {
        Face {
            quoted: self.quote_depth > 0,
            ..Face::plain()
        }
    }

    fn current_face(&self) -> Face {
        *self.faces.last().unwrap_or(&Face::plain())
    }

    /// Finalise the currently-accumulating block, if it holds any content,
    /// and reset to the ambient configuration. Idempotent: calling it
    /// again with nothing accumulated pushes nothing.
    fn finish(&mut self) {
        self.group_break();
        if !self.groups.iter().any(|g| !g.is_empty()) && self.groups.len() <= 1 {
            // Nothing was ever accumulated for this block — do not emit an
            // empty one. `groups.len() <= 1` because a single empty group
            // is the "never wrote anything" starting state, not a real
            // blank line inside real content.
            self.groups.clear();
            return;
        }
        let groups = std::mem::take(&mut self.groups);
        self.blocks.push(Block {
            kind: BlockKind::Flow,
            hard_split: self.hard_split,
            first_prefix: std::mem::take(&mut self.first_prefix),
            cont_prefix: std::mem::take(&mut self.cont_prefix),
            prefix_face: self.prefix_face,
            category: self.category,
            groups,
        });
        self.reset_ambient();
    }

    /// A thematic break: no `Start`/`End` pair of its own, so it is pushed
    /// directly rather than through the accumulate-then-`finish` path.
    /// Finalises whatever was open first, so it never absorbs a rule into
    /// a preceding paragraph's content.
    fn push_rule(&mut self) {
        self.finish();
        self.blocks.push(Block {
            kind: BlockKind::Rule,
            hard_split: false,
            first_prefix: String::new(),
            cont_prefix: String::new(),
            prefix_face: Face::plain(),
            category: Category::Other,
            groups: Vec::new(),
        });
    }

    /// Close the current line-group (a hard break, a verbatim line, or the
    /// block's own end) and start a new one. A soft break does not reach
    /// here: it folds into a space inside the current group.
    fn group_break(&mut self) {
        self.groups.push(std::mem::take(&mut self.group));
    }

    fn reset_ambient(&mut self) {
        self.first_prefix.clear();
        self.cont_prefix.clear();
        self.prefix_face = Face::plain();
        self.category = Category::Other;
        self.hard_split = false;
        self.groups.clear();
        self.group.clear();
        self.faces = vec![Face::plain()];
    }

    /// `Tag::Paragraph`. `finish()` above is a no-op when nothing was
    /// accumulated for the block still configured — which is exactly what
    /// happens for a **loose** list, where pulldown-cmark wraps every
    /// item's content in its own `Paragraph` even when the item is a
    /// single line: `Start(Item)` configures the marker and `Category::Item`,
    /// then `Start(Paragraph)` follows immediately with nothing yet
    /// written. Overwriting the prefix/category unconditionally here would
    /// discard the item's own marker and hanging indent the moment any
    /// item in the list forces the whole list loose — a single blank line
    /// anywhere in a bullet list is what CommonMark's own tightness rule
    /// hangs the whole list on. So a paragraph opening while the ambient
    /// state is still `Category::Item` (nothing having been finalised
    /// since) keeps that item's prefix, category, and face; only a
    /// paragraph opening at the top level or after a real block has
    /// closed resets to the ambient, unprefixed configuration.
    fn start_paragraph(&mut self) {
        self.finish();
        if self.category != Category::Item {
            let qp = quote_prefix(self.quote_depth);
            self.first_prefix = qp.clone();
            self.cont_prefix = qp;
            self.prefix_face = self.quoted_base();
            self.category = Category::Other;
        }
        self.hard_split = false;
        self.faces = vec![self.quoted_base()];
    }

    fn start_heading(&mut self, level: u8) {
        self.finish();
        // `spec-emphasis`: a heading is the boundary between one scenario
        // and the next, at any level, so an `AND` under a new heading can
        // never inherit the position of a clause above it.
        self.clause_position = None;
        let qp = quote_prefix(self.quote_depth);
        let face = Face {
            heading: Some(level),
            ..self.quoted_base()
        };
        self.first_prefix = format!("{qp}{}", "#".repeat(level as usize) + " ");
        self.cont_prefix = format!("{qp}{}", " ".repeat(level as usize + 1));
        self.prefix_face = face;
        self.category = Category::Other;
        self.hard_split = false;
        self.faces = vec![face];
    }

    /// `Tag::List(start)`: push a nesting frame. `start` is `Some(n)` for
    /// an ordered list beginning at `n`, `None` for a bullet list.
    fn start_list(&mut self, start: Option<u64>) {
        self.finish();
        let depth = self.list_stack.len();
        self.list_stack.push(ListFrame {
            ordered: start.is_some(),
            next: start.unwrap_or(1),
            depth,
        });
    }

    fn end_list(&mut self) {
        self.finish();
        self.list_stack.pop();
    }

    /// `Tag::Item`: the marker ([`BULLET`] or the list's own sequential number),
    /// two columns of indent per nesting level, and a hanging indent —
    /// the marker's width plus the nesting indent — for wrapped
    /// continuation lines. The marker itself never carries a face, even
    /// inside a block quote: `markdown-render` states plain segments for
    /// list markers specifically.
    fn start_item(&mut self) {
        self.finish();
        let frame = self
            .list_stack
            .last_mut()
            .expect("Item event outside an open List");
        let marker = if frame.ordered {
            let n = frame.next;
            frame.next += 1;
            format!("{n}. ")
        } else {
            BULLET.to_string()
        };
        let indent = "  ".repeat(frame.depth);
        let qp = quote_prefix(self.quote_depth);
        self.first_prefix = format!("{qp}{indent}{marker}");
        self.cont_prefix = format!("{qp}{}", " ".repeat(columns(&indent) + columns(&marker)));
        self.item_indent = indent;
        self.prefix_face = Face::plain();
        self.category = Category::Item;
        self.hard_split = false;
        let base = self.quoted_base();
        self.faces = vec![base];
    }

    /// `Event::TaskListMarker`: the three-character glyph `[✓]`/`[ ]` plus a
    /// space stands in place of the item's own bullet marker, so a
    /// task-list item's prefix costs four columns where a `• ` item's costs
    /// two.
    ///
    /// Both prefixes are rebuilt, not just the first.
    /// [`start_item`](Self::start_item) derives `cont_prefix` from the
    /// bullet marker's own width and runs **before** this event is seen, so
    /// an arm that rewrites `first_prefix` alone leaves a wrapped item's
    /// continuations hanging two columns in instead of four — and no width
    /// or totality assertion can detect that, because a shorter indent
    /// never overruns (design.md -> Decision 7).
    fn set_task_marker(&mut self, checked: bool) {
        // The guard is for totality over the event stream, not a live case:
        // `TaskListMarker` only ever arrives between `Start(Item)` and the
        // item's content, and `start_paragraph` deliberately preserves
        // `Category::Item` (see its `if self.category != Category::Item`), so
        // a loose list's item keeps its marker. If it ever did fire, the
        // effect would be the silent vanish this change exists to prevent —
        // which is why the reason is written here rather than inferred from
        // another function.
        if self.category != Category::Item {
            return;
        }
        let marker = if checked { CHECKED } else { UNCHECKED };
        let qp = quote_prefix(self.quote_depth);
        let indent = self.item_indent.clone();
        self.first_prefix = format!("{qp}{indent}{marker}");
        self.cont_prefix = format!("{qp}{}", " ".repeat(columns(&indent) + columns(marker)));
    }

    /// `Tag::Table(alignments)`. Keeps a list item's own marker for exactly
    /// the reason [`start_paragraph`](Self::start_paragraph) does — `finish`
    /// leaves `category` alone when nothing was accumulated — so a table
    /// nested in an item or a quote lays out in the columns its container
    /// leaves and carries the container's prefix on every line.
    fn start_table(&mut self, alignments: Vec<pulldown_cmark::Alignment>) {
        self.finish();
        if self.category != Category::Item {
            let qp = quote_prefix(self.quote_depth);
            self.first_prefix = qp.clone();
            self.cont_prefix = qp;
            self.prefix_face = self.quoted_base();
            self.category = Category::Other;
        }
        self.hard_split = false;
        self.groups.clear();
        self.group.clear();
        self.faces = vec![self.quoted_base()];
        self.table = Some(TableBuilder {
            alignments: alignments
                .into_iter()
                .map(|a| match a {
                    pulldown_cmark::Alignment::Right => Align::Right,
                    pulldown_cmark::Alignment::Center => Align::Center,
                    // `Left` and `None` — an unmarked column is padded on
                    // the right, which is what left alignment is.
                    _ => Align::Left,
                })
                .collect(),
            rows: Vec::new(),
            cells: Vec::new(),
            header: false,
        });
    }

    /// `Tag::TableHead` and `Tag::TableRow`: open a row, `header` set for
    /// the first of the two.
    fn start_table_row(&mut self, header: bool) {
        if let Some(t) = self.table.as_mut() {
            t.header = header;
            t.cells.clear();
        }
    }

    fn end_table_row(&mut self) {
        let base = self.quoted_base();
        if let Some(t) = self.table.as_mut() {
            let cells = std::mem::take(&mut t.cells);
            let header = t.header;
            t.rows.push(Row { cells, header });
            t.header = false;
        }
        self.faces = vec![base];
    }

    /// `Tag::TableCell`: the cell's runs accumulate in `group`. A header
    /// cell's base face carries `strong`, which is how the header row reads
    /// bold through the existing `Strong` role rather than through a face of
    /// its own (design.md -> Decision 6).
    fn start_table_cell(&mut self) {
        let mut face = self.quoted_base();
        if self.table.as_ref().is_some_and(|t| t.header) {
            face.strong = true;
        }
        self.group.clear();
        self.faces = vec![face];
    }

    fn end_table_cell(&mut self) {
        let cell = std::mem::take(&mut self.group);
        if let Some(t) = self.table.as_mut() {
            t.cells.push(cell);
        }
    }

    fn end_table(&mut self) {
        let Some(t) = self.table.take() else {
            return;
        };
        self.blocks.push(Block {
            kind: BlockKind::Table(Table {
                alignments: t.alignments,
                rows: t.rows,
            }),
            hard_split: false,
            first_prefix: std::mem::take(&mut self.first_prefix),
            cont_prefix: std::mem::take(&mut self.cont_prefix),
            prefix_face: self.prefix_face,
            category: self.category,
            groups: Vec::new(),
        });
        self.reset_ambient();
    }

    fn start_quote(&mut self) {
        self.finish();
        self.quote_depth += 1;
    }

    fn end_quote(&mut self) {
        self.finish();
        self.quote_depth -= 1;
    }

    /// `Tag::CodeBlock` (fenced or indented — pulldown-cmark strips the
    /// indent marker for the latter, so both arrive as the same verbatim
    /// `Text` content) and `Tag::HtmlBlock` share this configuration:
    /// verbatim, `code` true, hard-split rather than word-wrapped.
    fn start_verbatim_block(&mut self) {
        self.finish();
        let qp = quote_prefix(self.quote_depth);
        self.first_prefix = qp.clone();
        self.cont_prefix = qp;
        let face = Face {
            code: true,
            ..self.quoted_base()
        };
        self.prefix_face = face;
        self.category = Category::Other;
        self.hard_split = true;
        self.faces = vec![face];
    }

    fn push_text(&mut self, text: &str) {
        if let Some(alt) = self.image_alt.as_mut() {
            alt.push_str(text);
            return;
        }
        if self.hard_split {
            self.push_verbatim(text);
            return;
        }
        let face = self.current_face();
        self.group.push(Run {
            text: text.to_string(),
            face,
        });
    }

    /// Verbatim content for a code or HTML block: split at every `\n`
    /// (never word-wrapped here — `emit_block` hard-splits by width
    /// later), each line becoming its own group so it always starts a
    /// fresh output row. Handles a `\n` embedded inside one event and a
    /// line split across two events identically, and never manufactures a
    /// spurious trailing blank line from the final line's own newline.
    fn push_verbatim(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        let face = self.current_face();
        let ends_with_newline = text.ends_with('\n');
        let mut parts: Vec<&str> = text.split('\n').collect();
        if ends_with_newline {
            parts.pop();
        }
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                self.group_break();
            }
            if !part.is_empty() {
                self.group.push(Run {
                    text: (*part).to_string(),
                    face,
                });
            }
        }
        if ends_with_newline {
            self.group_break();
        }
    }

    fn push_code_span(&mut self, text: &str) {
        if let Some(alt) = self.image_alt.as_mut() {
            alt.push_str(text);
            return;
        }
        let mut face = self.current_face();
        face.code = true;
        self.group.push(Run {
            text: text.to_string(),
            face,
        });
    }

    fn push_faced(&mut self, extra: impl FnOnce(&mut Face)) {
        let mut face = self.current_face();
        extra(&mut face);
        self.faces.push(face);
    }

    fn pop_faced(&mut self) {
        self.faces.pop();
    }

    /// `Tag::Strong`. A run opening a list item — nothing yet pushed into
    /// the current `group` — is a *candidate* scenario-clause keyword;
    /// `group.len()` at this point is where its own runs will begin, and the
    /// candidacy is resolved once the span's whole text is known, at
    /// [`end_strong`](Self::end_strong) (design.md -> Decision 3). Every
    /// other `Strong` — mid-run, inside a table cell, in a paragraph outside
    /// a list — pushes `None`, which `end_strong` leaves untouched.
    fn start_strong(&mut self) {
        let candidate = self.category == Category::Item && self.group.is_empty();
        self.strong_starts
            .push(candidate.then_some(self.group.len()));
        self.push_faced(|face| face.strong = true);
    }

    /// `TagEnd::Strong`. Resolves a candidacy `start_strong` opened:
    /// `specs::clause_of` classifies the span's whole accumulated text —
    /// never a prefix, never trimmed — and a hit faces every run the span
    /// pushed (ordinarily exactly one) with the clause's lifecycle role,
    /// remembering it for a later `AND` or inheriting the one already
    /// remembered. A miss, or no candidacy at all, leaves every face
    /// `end_strong` sees exactly as `push_text` set it.
    fn end_strong(&mut self) {
        self.pop_faced();
        let Some(start) = self.strong_starts.pop().flatten() else {
            return;
        };
        let text: String = self.group[start..]
            .iter()
            .map(|r| r.text.as_str())
            .collect();
        let role = match clause_of(&text) {
            Some(Clause::Opens(role)) => {
                self.clause_position = Some(role);
                role
            }
            Some(Clause::Continues) => self
                .clause_position
                .unwrap_or(crate::tasks::LabelRole::Other),
            None => return,
        };
        for run in &mut self.group[start..] {
            run.face.label = Some(role);
        }
    }

    fn start_image(&mut self) {
        let mut face = self.current_face();
        face.link = true;
        self.image_face = face;
        self.image_alt = Some(String::new());
    }

    fn end_image(&mut self) {
        let Some(alt) = self.image_alt.take() else {
            return;
        };
        let text = if alt.is_empty() {
            "[img]".to_string()
        } else {
            alt
        };
        self.group.push(Run {
            text,
            face: self.image_face,
        });
    }

    fn end(mut self) -> Vec<Block> {
        // A source truncated inside a table leaves the builder open; close
        // it rather than dropping the rows, on the same totality terms
        // `finish` closes a truncated paragraph.
        if self.table.is_some() {
            self.end_table();
        }
        self.finish();
        self.blocks
    }
}

/// Fold `source`'s event stream into a `Vec<Block>` — plain data carrying
/// its own indent, marker, and inline runs, but nothing width-dependent.
/// Every block-level construct `Options::empty()` can produce is handled:
/// paragraphs, headings, lists (nested, ordered from their own start
/// value), code blocks (fenced or indented, verbatim), block quotes
/// (nested), thematic breaks, raw HTML (block and inline), and — with
/// `ENABLE_TABLES` and `ENABLE_TASKLISTS` on — GFM pipe tables and
/// task-list items. A footnote is not modelled by this option set at all:
/// it arrives as ordinary paragraph text, which is the literal-text
/// degraded state `markdown-render` states.
///
/// The option set is exactly `ENABLE_TABLES | ENABLE_STRIKETHROUGH |
/// ENABLE_TASKLISTS` and nothing else: a further flag turned on here
/// starts emitting events into the wildcards below, where a construct
/// **vanishes** rather than degrading to its literal text. This change is
/// that warning's own worked example, which is why `ENABLE_TASKLISTS` and
/// the `Event::TaskListMarker` arm landed in one edit.
fn fold(source: &str) -> Vec<Block> {
    let mut f = Folder::new();
    for event in Parser::new_ext(
        source,
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS,
    ) {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => f.start_paragraph(),
                Tag::Heading { level, .. } => f.start_heading(level as u8),
                Tag::List(start) => f.start_list(start),
                Tag::Item => f.start_item(),
                Tag::BlockQuote(_) => f.start_quote(),
                Tag::CodeBlock(_) | Tag::HtmlBlock => f.start_verbatim_block(),
                Tag::Emphasis => f.push_faced(|face| face.emphasis = true),
                Tag::Strong => f.start_strong(),
                Tag::Link { .. } => f.push_faced(|face| face.link = true),
                Tag::Strikethrough => f.push_faced(|face| face.strikethrough = true),
                Tag::Image { .. } => f.start_image(),
                Tag::Table(alignments) => f.start_table(alignments),
                Tag::TableHead => f.start_table_row(true),
                Tag::TableRow => f.start_table_row(false),
                Tag::TableCell => f.start_table_cell(),
                // `FootnoteDefinition`, the definition-list tags,
                // `Superscript`, `Subscript`, and `MetadataBlock` cannot be
                // produced by this option set. `Strikethrough` can, and is
                // handled above. The wildcard is the stated default: total
                // over the enum, and a future pulldown-cmark variant reaches
                // it rather than a missing-arm compile error changing this
                // module's shape.
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Paragraph | TagEnd::Heading(_) => f.finish(),
                TagEnd::List(_) => f.end_list(),
                TagEnd::Item | TagEnd::CodeBlock | TagEnd::HtmlBlock => f.finish(),
                TagEnd::BlockQuote(_) => f.end_quote(),
                TagEnd::Strong => f.end_strong(),
                TagEnd::Emphasis | TagEnd::Link | TagEnd::Strikethrough => f.pop_faced(),
                TagEnd::Image => f.end_image(),
                TagEnd::Table => f.end_table(),
                TagEnd::TableHead | TagEnd::TableRow => f.end_table_row(),
                TagEnd::TableCell => f.end_table_cell(),
                _ => {}
            },
            Event::Text(text) => f.push_text(&text),
            Event::Code(text) => f.push_code_span(&text),
            // Inline HTML is verbatim and `code`-faced, on the same terms
            // as an inline code span, and shares its ordinary word-wrap
            // path: a long inline tag hard-splits via the oversized-token
            // rule rather than needing its own hard-split block.
            Event::InlineHtml(text) => f.push_code_span(&text),
            // An HTML *block*'s lines arrive through `hard_split`, set by
            // `start_verbatim_block`; `push_text` routes to
            // `push_verbatim` whenever that flag is set.
            Event::Html(text) => f.push_text(&text),
            // A soft break folds into a single space so the paragraph
            // reflows as one unit at the region width; a hard break is the
            // author's explicit opt-out and still starts a new rendered
            // line (design.md -> Decision 1).
            Event::SoftBreak => f.push_text(" "),
            Event::HardBreak => f.group_break(),
            Event::Rule => f.push_rule(),
            Event::TaskListMarker(checked) => f.set_task_marker(checked),
            // `FootnoteReference`, `InlineMath`, and `DisplayMath` cannot be
            // produced by this option set — `ENABLE_FOOTNOTES` stays off
            // precisely so a footnote keeps arriving as literal text
            // instead. The wildcard is the stated default: total over the
            // enum, and a future pulldown-cmark variant reaches it rather
            // than a missing-arm compile error changing this module's shape.
            _ => {}
        }
    }
    f.end()
}

/// Lay out `blocks` at `width`, inserting exactly one blank `Line` between
/// two adjacent blocks unless both are list items of the same list.
fn layout(blocks: &[Block], width: u16) -> Vec<Line> {
    let mut out = Vec::new();
    for (i, block) in blocks.iter().enumerate() {
        if i > 0 {
            let same_list_items =
                block.category == Category::Item && blocks[i - 1].category == Category::Item;
            if !same_list_items {
                out.push(Line::blank());
            }
        }
        emit_block(block, width, &mut out);
    }
    out
}

fn emit_block(block: &Block, width: u16, out: &mut Vec<Line>) {
    if matches!(block.kind, BlockKind::Rule) {
        out.push(Line {
            segments: vec![Segment {
                text: RULE.repeat(width as usize),
                face: Face::plain(),
            }],
        });
        return;
    }
    let prefix_len = columns(&block.first_prefix) as u16;
    let content_width = width.saturating_sub(prefix_len);
    if content_width == 0 {
        let truncated = truncate_columns(&block.first_prefix, width as usize).to_string();
        if !truncated.is_empty() {
            out.push(Line {
                segments: vec![Segment {
                    text: truncated,
                    face: block.prefix_face,
                }],
            });
        } else {
            out.push(Line::blank());
        }
        return;
    }

    if let BlockKind::Table(table) = &block.kind {
        emit_table(block, table, content_width, out);
        return;
    }

    let mut first_line_of_block = true;
    for group in &block.groups {
        let wrapped = if block.hard_split {
            hard_split_group(group, content_width as usize)
        } else {
            wrap_prose(group, content_width as usize)
        };
        for segs in wrapped {
            let prefix = if first_line_of_block {
                &block.first_prefix
            } else {
                &block.cont_prefix
            };
            let mut segments = Vec::new();
            if !prefix.is_empty() {
                segments.push(Segment {
                    text: prefix.clone(),
                    face: block.prefix_face,
                });
            }
            segments.extend(segs);
            out.push(Line { segments });
            first_line_of_block = false;
        }
    }
}

/// The natural width of every declared column: the greatest [`columns`] of
/// any cell in it, at least `1` so a column of only empty cells still
/// carries a position the reader can see.
fn natural_widths(table: &Table, n: usize) -> Vec<usize> {
    let mut nat = vec![1usize; n];
    for row in &table.rows {
        for (j, cell) in row.cells.iter().enumerate().take(n) {
            let text: String = cell.iter().map(|r| r.text.as_str()).collect();
            nat[j] = nat[j].max(columns(&text));
        }
    }
    nat
}

/// Max-min fair allocation of `avail` content columns over the natural
/// widths `nat` (design.md -> Decision 4): every column keeps its natural
/// width while the budget lasts, then only the greedy ones are capped —
/// `w[j] = min(nat[j], c)` for the largest `c` that fits — and the columns
/// left over are handed out one at a time, by ascending index, to columns
/// still short of their natural width.
///
/// A proportional shrink is the alternative, and it squeezes a four-column
/// `Gate` label to one to feed a sixty-column prose column, which is the
/// opposite of useful: the short columns are what a reader scans by.
///
/// Every returned width is at least `1`, because the caller only reaches
/// here when `avail >= nat.len()`, so `c = 1` always fits.
fn allocate(nat: &[usize], avail: usize) -> Vec<usize> {
    if nat.iter().sum::<usize>() <= avail {
        return nat.to_vec();
    }
    let cap = nat.iter().copied().max().unwrap_or(1);
    let mut c = 1usize;
    for candidate in 1..=cap {
        if nat.iter().map(|x| (*x).min(candidate)).sum::<usize>() <= avail {
            c = candidate;
        } else {
            break;
        }
    }
    let mut w: Vec<usize> = nat.iter().map(|x| (*x).min(c)).collect();
    let mut left = avail - w.iter().sum::<usize>();
    for (j, width) in w.iter_mut().enumerate() {
        if left == 0 {
            break;
        }
        if *width < nat[j] {
            *width += 1;
            left -= 1;
        }
    }
    w
}

/// Lay `table` out at `width` columns and push the result into `out`,
/// carrying `block`'s own prefix on every line — its first prefix on the
/// first line and its continuation prefix on the rest, exactly as a flow
/// block's lines do, so a table nested in a quote or a list item stays
/// inside the columns its container leaves.
///
/// Total: a table declaring no column, and a zero width, each produce no
/// line rather than dividing by zero. No `&str` source yields
/// `Tag::Table([])` — `||` and `|-|` both declare one column — so that
/// guard is an invariant rather than an observable rendering.
fn emit_table(block: &Block, table: &Table, width: u16, out: &mut Vec<Line>) {
    let n = table.alignments.len();
    if n == 0 || width == 0 {
        return;
    }
    let width = width as usize;
    let avail = width.saturating_sub(3 * n + 1);
    // Below `4n + 1` columns the pipe grammar cannot carry even one content
    // column per column (design.md -> Decision 7).
    let rendered = if avail < n {
        narrow_rows(table, width)
    } else {
        pipe_rows(table, &allocate(&natural_widths(table, n), avail))
    };
    let mut first_line_of_block = true;
    for segs in rendered {
        let prefix = if first_line_of_block {
            &block.first_prefix
        } else {
            &block.cont_prefix
        };
        let mut segments = Vec::new();
        if !prefix.is_empty() {
            segments.push(Segment {
                text: prefix.clone(),
                face: block.prefix_face,
            });
        }
        segments.extend(segs);
        out.push(Line { segments });
        first_line_of_block = false;
    }
}

/// The pipe grammar: a row line is a leading `|`, then for each column a
/// padding space, the cell's content for that line laid out in exactly
/// `w[j]` columns, a padding space, and the `|` that closes it — so a row
/// line holds `n + 1` pipes and measures exactly `3n + 1 + sum(w)`. The
/// delimiter line, emitted once immediately after the header row's last
/// line, is `-` repeated `w[j] + 2` per column between the same pipes, so
/// its pipes fall exactly under the row lines' (design.md -> Decision 3).
///
/// A cell too wide for its column **wraps** inside it, by the same
/// [`wrap_prose`] every paragraph uses, and is never truncated: the detail
/// region has no horizontal scroll to recover what a cut would discard
/// (design.md -> Decision 5). A row is therefore as tall as its tallest
/// cell, and the rendered line count and the source row count legitimately
/// diverge.
fn pipe_rows(table: &Table, w: &[usize]) -> Vec<Vec<Segment>> {
    let n = w.len();
    let mut out: Vec<Vec<Segment>> = Vec::new();
    for row in &table.rows {
        let empty: Cell = Vec::new();
        let wrapped: Vec<Vec<Vec<Segment>>> = (0..n)
            .map(|j| {
                let cell = row.cells.get(j).unwrap_or(&empty);
                let lines = wrap_prose(cell, w[j]);
                if lines.is_empty() {
                    vec![Vec::new()]
                } else {
                    lines
                }
            })
            .collect();
        let height = wrapped.iter().map(Vec::len).max().unwrap_or(1).max(1);
        for k in 0..height {
            let mut segs: Vec<Segment> = Vec::new();
            append(&mut segs, TABLE_SEP, Face::plain());
            for (j, cell_lines) in wrapped.iter().enumerate() {
                append(&mut segs, " ", Face::plain());
                let line: &[Segment] = cell_lines.get(k).map_or(&[], Vec::as_slice);
                let used: usize = line.iter().map(|s| columns(&s.text)).sum();
                let pad = w[j].saturating_sub(used);
                // The alignment applies to every line of a wrapped cell,
                // not to its first alone; a centred cell's odd column goes
                // to the right.
                let (lead, trail) = match table.alignments[j] {
                    Align::Left => (0, pad),
                    Align::Right => (pad, 0),
                    Align::Center => (pad / 2, pad - pad / 2),
                };
                if lead > 0 {
                    append(&mut segs, &" ".repeat(lead), Face::plain());
                }
                for s in line {
                    append(&mut segs, &s.text, s.face);
                }
                if trail > 0 {
                    append(&mut segs, &" ".repeat(trail), Face::plain());
                }
                append(&mut segs, " ", Face::plain());
                append(&mut segs, TABLE_SEP, Face::plain());
            }
            out.push(segs);
        }
        if row.header {
            // The delimiter line is a row line's own shape with every padding
            // space and content column replaced by `─` and the separators by
            // `├`/`┼`/`┤`, so its junctions fall under the row lines' `│` by
            // construction rather than by a second arithmetic
            // (design.md -> Decision 3).
            let mut segs: Vec<Segment> = Vec::new();
            append(&mut segs, TABLE_RULE_LEFT, Face::plain());
            for (j, cell_width) in w.iter().enumerate() {
                append(&mut segs, &RULE.repeat(cell_width + 2), Face::plain());
                let sep = if j + 1 == n {
                    TABLE_RULE_RIGHT
                } else {
                    TABLE_RULE_MID
                };
                append(&mut segs, sep, Face::plain());
            }
            out.push(segs);
        }
    }
    out
}

/// The narrow fallback (design.md -> Decision 7): one cell per line, in
/// row-major order, each wrapped to the full `width` by the prose rule,
/// header cells still carrying `strong`, and an empty cell emitting
/// nothing. No pipe, no padding, and no delimiter line. It preserves every
/// character and never exceeds the region, which is what a table in a
/// three-column region can still be asked for.
fn narrow_rows(table: &Table, width: usize) -> Vec<Vec<Segment>> {
    let mut out = Vec::new();
    for row in &table.rows {
        for cell in &row.cells {
            if cell.iter().all(|r| r.text.is_empty()) {
                continue;
            }
            out.extend(wrap_prose(cell, width));
        }
    }
    out
}

/// Split `s` at a grapheme-cluster boundary into a prefix whose
/// [`columns`] is at most `width`, and the remainder. Never panics and
/// never splits a cluster, for any `s` and any `width`.
///
/// When even the first cluster does not fit — it alone measures more than
/// `width` columns — it is **dropped** rather than emitted, per
/// `markdown-render`'s carve-out for an over-wide token: the returned
/// prefix is empty and the remainder skips the dropped cluster, so the
/// caller always makes progress rather than looping on it forever. This is
/// the one case in which content is lost; every other call returns a
/// prefix that is a genuine byte-prefix of `s`.
fn split_at_columns(s: &str, width: usize) -> (&str, &str) {
    let prefix = truncate_columns(s, width);
    if !prefix.is_empty() || s.is_empty() {
        return (prefix, &s[prefix.len()..]);
    }
    // `truncate_columns` returned empty on non-empty `s`: the first
    // grapheme cluster alone is wider than `width`. Find its byte length
    // by growing the budget one column at a time until something fits —
    // bounded by `s`'s own total columns, at which point `truncate_columns`
    // returns `s` whole, so this always terminates.
    let total = columns(s);
    let mut probe = width + 1;
    loop {
        let candidate = truncate_columns(s, probe);
        if !candidate.is_empty() {
            return ("", &s[candidate.len()..]);
        }
        if probe >= total {
            return ("", "");
        }
        probe += 1;
    }
}

/// One atom of a word-wrappable stream: a [`Word`], or a space (a break
/// opportunity, never itself rendered — a single space is synthesised
/// between two words that need one).
enum Atom {
    Word(Word),
    /// A space, carrying the face of the run it was found in — the run
    /// that contains a word and its own adjacent space is what a
    /// multi-word faced phrase (`the design`, all one `Link` run) is
    /// built from, so the space merges into the same segment as its
    /// neighbours rather than forcing a break at every face boundary.
    Space(Face),
}

/// A maximal run of non-space characters, which MAY span several faces.
/// An inline face change with no space between it and its neighbours —
/// `*bravo*,`, `` `bravo`sierra ``, `**bravo**tango` — is **not** a break
/// opportunity: `markdown-render` requires wrapping to be greedy at ASCII
/// spaces, and a run boundary is not one. So the word spans every face it
/// touches and is measured, wrapped, and hard-split as one.
///
/// `text` is the run's characters, its pieces concatenated, and is the
/// subject of every width measurement and of the hard split, so a grapheme
/// cluster spanning a face boundary is measured and split as one rather
/// than as two half-clusters. `faces` holds each piece's start byte offset
/// in `text` and that piece's face, in order: never empty for a non-empty
/// word, and always opening at offset 0.
struct Word {
    text: String,
    faces: Vec<(usize, Face)>,
}

impl Word {
    fn new() -> Word {
        Word {
            text: String::new(),
            faces: Vec::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Append `ch` at `face`, opening a new face piece only where the face
    /// actually changes — so a single-faced word carries exactly one piece
    /// and produces exactly the segments it produced before words spanned
    /// faces at all.
    fn push(&mut self, ch: char, face: Face) {
        if self.faces.last().is_none_or(|&(_, f)| f != face) {
            self.faces.push((self.text.len(), face));
        }
        self.text.push(ch);
    }

    /// Append `self.text[from..to]` to `current`, one piece at a time and
    /// each through [`append`], so two adjacent same-face pieces still
    /// merge into one segment exactly as two adjacent same-face words do.
    /// Both offsets are byte offsets into `text` and both fall on piece
    /// boundaries or inside a piece; an empty intersection contributes
    /// nothing.
    fn append_range(&self, current: &mut Vec<Segment>, from: usize, to: usize) {
        for (index, &(start, face)) in self.faces.iter().enumerate() {
            let end = self
                .faces
                .get(index + 1)
                .map_or(self.text.len(), |&(next, _)| next);
            let (lo, hi) = (start.max(from), end.min(to));
            if lo < hi {
                append(current, &self.text[lo..hi], face);
            }
        }
    }
}

fn atomize(group: &[Run]) -> Vec<Atom> {
    let mut atoms = Vec::new();
    // Deliberately outside the `group` loop: a word is closed by a space
    // and by the end of the stream, never by a run boundary.
    let mut current = Word::new();
    for run in group {
        for ch in run.text.chars() {
            if ch == ' ' {
                if !current.is_empty() {
                    atoms.push(Atom::Word(std::mem::replace(&mut current, Word::new())));
                }
                atoms.push(Atom::Space(run.face));
            } else {
                current.push(ch, run.face);
            }
        }
    }
    if !current.is_empty() {
        atoms.push(Atom::Word(current));
    }
    atoms
}

/// Append `text` at `face` to `current`, merging into its last segment
/// when the face matches rather than starting a new one — so two adjacent
/// same-face words (and the space between them) read as one segment, on
/// the same terms as `Line::text()` reading through every segment.
fn append(current: &mut Vec<Segment>, text: &str, face: Face) {
    if let Some(last) = current.last_mut()
        && last.face == face
    {
        last.text.push_str(text);
        return;
    }
    current.push(Segment {
        text: text.to_string(),
        face,
    });
}

/// Greedy word wrap at `width` (already the block's content width, never
/// zero — `emit_block` handles the zero case before calling this). A word
/// longer than `width`, even on a line of its own, is hard-split at
/// exactly `width` characters and continued on the next line. A separator
/// space carries the face `atomize` tagged it with, so a multi-word faced
/// phrase from one run (`the design`, all `Link`) merges into a single
/// segment rather than breaking at every word.
fn wrap_prose(group: &[Run], width: usize) -> Vec<Vec<Segment>> {
    let atoms = atomize(group);
    let mut lines: Vec<Vec<Segment>> = Vec::new();
    let mut current: Vec<Segment> = Vec::new();
    let mut current_len = 0usize;
    let mut pending_space: Option<Face> = None;

    for atom in atoms {
        match atom {
            Atom::Space(face) => {
                if current_len > 0 {
                    pending_space = Some(face);
                }
            }
            Atom::Word(word) => {
                // The byte offset into `word.text` of the part not yet
                // placed, advanced only by the hard split — every other
                // path places the whole remainder and breaks.
                let mut start = 0usize;
                loop {
                    let remaining = &word.text[start..];
                    let word_cols = columns(remaining);
                    if current_len == 0 && word_cols > width {
                        let (chunk, rest) = split_at_columns(remaining, width);
                        if !chunk.is_empty() {
                            word.append_range(&mut current, start, start + chunk.len());
                        }
                        lines.push(std::mem::take(&mut current));
                        current_len = 0;
                        pending_space = None;
                        // `rest` is a suffix of `word.text`, including in
                        // the dropped-cluster case, where it is shorter
                        // than `remaining` — so this always advances.
                        start = word.text.len() - rest.len();
                        if start >= word.text.len() {
                            break;
                        }
                        continue;
                    }
                    let sep = if pending_space.is_some() && current_len > 0 {
                        1
                    } else {
                        0
                    };
                    if current_len + sep + word_cols <= width {
                        if let Some(sep_face) = pending_space.take() {
                            append(&mut current, " ", sep_face);
                            current_len += 1;
                        }
                        word.append_range(&mut current, start, word.text.len());
                        current_len += word_cols;
                        pending_space = None;
                        break;
                    }
                    lines.push(std::mem::take(&mut current));
                    current_len = 0;
                    pending_space = None;
                }
            }
        }
    }
    if current_len > 0 || !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Hard-split a verbatim group (exactly one `Run`, a single source line)
/// into `width`-wide chunks, never at a word boundary.
fn hard_split_group(group: &[Run], width: usize) -> Vec<Vec<Segment>> {
    let Some(run) = group.first() else {
        return vec![Vec::new()];
    };
    if run.text.is_empty() {
        return vec![Vec::new()];
    }
    let mut out = Vec::new();
    let mut remaining = run.text.as_str();
    while !remaining.is_empty() {
        let (chunk, rest) = split_at_columns(remaining, width);
        out.push(if chunk.is_empty() {
            Vec::new()
        } else {
            vec![Segment {
                text: chunk.to_string(),
                face: run.face,
            }]
        });
        remaining = rest;
    }
    out
}

/// Turn `source` into the lines a `width`-column-wide region would show.
/// A pure total transformation: empty for an empty source or a zero
/// width, and no line exceeds `width` characters (counted, like
/// `ui::list`, in `char`s).
pub fn lines(source: &str, width: u16) -> Vec<Line> {
    if source.is_empty() || width == 0 {
        return Vec::new();
    }
    let blocks = fold(source);
    layout(&blocks, width)
}

/// Whether `line` is nothing but spaces, tabs, and three or more of the
/// same `-`, `=`, `_`, or `*` character — the pattern that, unescaped,
/// would read as a thematic break or (given a preceding paragraph line) a
/// setext heading underline.
fn is_whole_line_break_run(line: &str) -> bool {
    let mut significant = line.chars().filter(|c| *c != ' ' && *c != '\t');
    let first = match significant.next() {
        Some(c) if matches!(c, '-' | '=' | '_' | '*') => c,
        _ => return false,
    };
    let mut count = 1;
    for c in significant {
        if c != first {
            return false;
        }
        count += 1;
    }
    count >= 3
}

/// The byte offset, if any, at which `text` needs a CommonMark backslash
/// escape so that none of its own leading characters can open a block.
/// Only ASCII punctuation can be escaped this way — a backslash prepended
/// to a digit is emitted verbatim rather than consumed — so an
/// ordered-list opener is escaped *after* its digit run, never before it;
/// every other trigger is escaped at position `0` (design.md -> Decision 4).
fn leading_block_escape(text: &str) -> Option<usize> {
    if text.is_empty() {
        return None;
    }

    // ATX heading: one to six `#` followed by a space or the end of the
    // first line.
    let hashes = text.chars().take_while(|&c| c == '#').count();
    if (1..=6).contains(&hashes) {
        let after = &text[hashes..];
        if after.is_empty() || after.starts_with(' ') || after.starts_with('\n') {
            return Some(0);
        }
    }

    if let Some(first) = text.chars().next() {
        // Bullet list marker: `-`, `*`, or `+` followed by a space.
        if matches!(first, '-' | '*' | '+') && text[first.len_utf8()..].starts_with(' ') {
            return Some(0);
        }
        // Block quote.
        if first == '>' {
            return Some(0);
        }
    }

    // Fenced code block opener: three or more backticks or tildes.
    for fence in ['`', '~'] {
        if text.chars().take_while(|&c| c == fence).count() >= 3 {
            return Some(0);
        }
    }

    // Thematic break / setext underline: the whole first line and nothing
    // else.
    let first_line = text.split('\n').next().unwrap_or("");
    if is_whole_line_break_run(first_line) {
        return Some(0);
    }

    // Ordered list start: a digit run followed by `.` or `)` and then a
    // space or the end of the first line. The escape goes after the
    // digits, not before them.
    let digits = text.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 {
        let after = &text[digits..];
        if let Some(marker) = after.chars().next()
            && (marker == '.' || marker == ')')
        {
            let past_marker = &after[marker.len_utf8()..];
            if past_marker.is_empty()
                || past_marker.starts_with(' ')
                || past_marker.starts_with('\n')
            {
                return Some(digits);
            }
        }
    }

    None
}

/// `text` with every line's own block opener escaped and every line's
/// leading whitespace dropped, so that [`fold`] can recognise no block
/// construct anywhere in it.
///
/// Two rules beyond [`leading_block_escape`]'s own, each measured rather
/// than reasoned about:
///
/// **Every line, not only the first.** A setext underline lives on the
/// line *after* the text it promotes, and a blank line lets a later line
/// open a block of its own, so escaping only the first line leaves
/// `"foo\n==="` rendering as an `h1` and `"para\n\n# heading"` carrying a
/// `heading` face — both forbidden by this function's own requirement.
///
/// **Leading whitespace is dropped, not escaped.** A backslash placed
/// before a space is emitted literally rather than consumed, so a
/// whitespace-led fragment escaped in place renders a **visible** `\`;
/// and four columns of leading whitespace open an indented code block,
/// which `"\t### heading"` was measured to reach. A paragraph's leading
/// whitespace carries no text of its own — CommonMark strips it — so
/// dropping it neither loses a character a reader would see nor leaves a
/// block opener behind. Trailing whitespace is untouched, two trailing
/// spaces being a hard break this function is required to honour.
fn escape_fragment(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let line = line.trim_start();
        match leading_block_escape(line) {
            Some(at) => {
                out.push_str(&line[..at]);
                out.push('\\');
                out.push_str(&line[at..]);
            }
            None => out.push_str(line),
        }
    }
    out
}

/// Render `text` as the single paragraph it is, on exactly the terms
/// [`lines`] renders a paragraph it found inside a document: the same
/// faces, the same wrapping, the same `Line`/`Segment`/`Face` types.
/// `fold` has no inline-only entry point, so `inline` escapes whichever
/// character would open a block — on every line, [`escape_fragment`]
/// carrying the reasons — then folds and lays the escaped text out
/// exactly as `lines` would, recognising no block construct and setting
/// no `heading` face on any segment it returns (design.md -> Decision 4).
pub fn inline(text: &str, width: u16) -> Vec<Line> {
    if width == 0 || text.trim().is_empty() {
        return Vec::new();
    }
    let blocks = fold(&escape_fragment(text));
    layout(&blocks, width)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_of(lines: &[Line]) -> Vec<String> {
        lines.iter().map(Line::text).collect()
    }

    #[test]
    fn paragraph_wraps_at_58_and_78() {
        let source = "alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima mike november oscar papa";
        let at58 = lines(source, 58);
        assert_eq!(
            text_of(&at58),
            vec![
                "alpha bravo charlie delta echo foxtrot golf hotel india".to_string(),
                "juliett kilo lima mike november oscar papa".to_string(),
            ]
        );
        let at78 = lines(source, 78);
        let first78 =
            "alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima mike";
        assert_eq!(columns(first78), 78);
        assert_eq!(
            text_of(&at78),
            vec![first78.to_string(), "november oscar papa".to_string()]
        );
        for line in at58.iter().chain(at78.iter()) {
            for seg in &line.segments {
                assert_eq!(seg.face, Face::plain());
            }
        }
    }

    #[test]
    fn empty_source_and_zero_width_produce_no_lines() {
        assert_eq!(lines("", 58), Vec::new());
        assert_eq!(lines("", 78), Vec::new());
        assert_eq!(lines("# Proposal\n", 0), Vec::new());
        assert_eq!(lines("", 0), Vec::new());
    }

    /// `markdown-render` -> "Wrapping SHALL be greedy at ASCII spaces": an
    /// inline face change with no space between is **not** a break
    /// opportunity, so `*bravo*,`, `` `bravo`sierra `` and `**bravo**tango`
    /// each wrap as one word rather than leaving their differently-faced
    /// tail to open the next line.
    ///
    /// Swept over a growing pad so the wrap point lands on every column of
    /// the glued word at both mandated widths, rather than on whichever one
    /// column a single fixture happens to produce.
    #[test]
    fn a_face_change_without_a_space_is_not_a_break_opportunity() {
        for width in [58, 78] {
            for pad in 0..40usize {
                let prefix = "pad ".repeat(pad);
                for source in [
                    format!("{prefix}alpha *bravo*, charlie"),
                    format!("{prefix}alpha `bravo`sierra charlie"),
                    format!("{prefix}alpha **bravo**tango charlie"),
                ] {
                    for line in lines(&source, width) {
                        let text = line.text();
                        for orphan in [",", "sierra", "tango"] {
                            assert!(
                                !text.starts_with(orphan),
                                "width {width}, pad {pad}: {text:?} opens with {orphan:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    fn composite_fixture() -> String {
        // Heading, paragraph, bullet list, nested list, fenced code, block
        // quote, thematic break, link, and a three-column table whose widest
        // cell exceeds the narrow interior — this test only needs to prove
        // totality and width compliance, not per-construct styling (the
        // dedicated tests below own that). Each block carries one
        // distinctive one-word marker so a check can find it regardless of
        // where a wrap point lands.
        //
        // The table is three columns, so `4n + 1` is 13 and the swept widths
        // 1, 2, 3, and 10 all fall below it: the degenerate one-cell-per-line
        // path is measured by this sweep rather than assumed.
        //
        // The nested block quote and the three task-list items — checked,
        // unchecked, and one nested inside a quote — are here rather than in
        // a second all-constructs document beside this one, so every sweep
        // and every set-difference check reads the same fixture. A task-list
        // item's prefix is four columns, which the swept widths 1, 2, and 3
        // all fall under: `width.saturating_sub(prefix)` is measured at
        // underflow rather than assumed total.
        "# headingword marker\n\
         \n\
         paragraphword marker text padded out with extra words so the line wraps at both widths under test here today and ~~struckword~~ too\n\
         \n\
         - bulletword marker text padded with extra words so the item wraps under both widths under test\n  - nestedword marker text padded with extra words so the item wraps under both widths under test\n\
         \n\
         ```sh\n\
         codeword marker text padded with extra words so the block wraps under both widths under test today\n\
         ```\n\
         \n\
         > quoteword marker text padded with extra words so the quote wraps under both widths under test today\n\
         \n\
         > > nestedquoteword marker text padded with extra words so the nested quote wraps under both widths\n\
         \n\
         > - [ ] quotedtaskword marker text padded with extra words so the item wraps under both widths\n\
         \n\
         - [x] checkedtaskword marker text padded with extra words so the item wraps under both widths\n\
         - [ ] uncheckedtaskword marker text padded with extra words so the item wraps under both widths\n\
         \n\
         ---\n\
         \n\
         See [linkword marker](design.md) for more padded words so this paragraph also wraps under test today.\n\
         \n\
         | tableword | col2 | col3 |\n\
         |---|:--:|---:|\n\
         | a | b | this cell is padded out with enough extra words that it exceeds the narrow interior |\n"
            .to_string()
    }

    /// One segment as HEAD emitted it: its text and its seven pre-change face
    /// fields, transcribed from
    /// `openspec/changes/tasks-emphasis/notes/head-output.md` — each row naming
    /// only the fields HEAD set, the rest arriving from `Face::plain()`. The
    /// comparison is against a whole `Face`, so the two fields this change adds
    /// are compared too, at their zero, which is the claim.
    struct Recorded {
        text: &'static str,
        face: Face,
    }

    fn rec(text: &'static str, face: Face) -> Recorded {
        Recorded { text, face }
    }

    /// The group-3 document: every construct `markdown-render`'s new scenario
    /// names — a heading, a paragraph, a bullet list, a fenced code block, a
    /// block quote, a link, a struck run, a table, a checked and an unchecked
    /// task-list item — plus the literal paragraph `VERIFY: this is prose, not
    /// a task`, which is the fixture's whole point: a proposal that opens with
    /// the word `VERIFY:` is prose, and recognising a label is
    /// `tasks-checklist`'s job on the checklist path and never this renderer's.
    fn face_field_fixture() -> &'static str {
        "# Heading\n\
         \n\
         A paragraph of prose.\n\
         \n\
         - alpha\n\
         - bravo\n\
         \n\
         ```sh\n\
         cargo test\n\
         ```\n\
         \n\
         > quoted line\n\
         \n\
         A [link](https://example.com) here.\n\
         \n\
         A ~~struck~~ run.\n\
         \n\
         | a | b |\n\
         |---|---|\n\
         | 1 | 2 |\n\
         \n\
         - [x] done item\n\
         - [ ] open item\n\
         \n\
         VERIFY: this is prose, not a task\n"
    }

    /// `markdown-render` :: "The markdown path sets neither new face field".
    ///
    /// The recorded half is a **literal** table captured from HEAD before this
    /// change — `openspec/changes/tasks-emphasis/notes/head-output.md`, task
    /// 0.5 — one row per emitted segment, its text and its seven pre-change
    /// face fields. Comparing against a fresh call of the function under test
    /// could not fail; this can, and it is what carries the claim that adding
    /// two fields moved no rendered output.
    #[test]
    fn the_markdown_path_sets_neither_new_face_field() {
        let recorded = [
            rec(
                "# ",
                Face {
                    heading: Some(1),
                    ..Face::plain()
                },
            ),
            rec(
                "Heading",
                Face {
                    heading: Some(1),
                    ..Face::plain()
                },
            ),
            rec("A paragraph of prose.", Face::plain()),
            rec("• ", Face::plain()),
            rec("alpha", Face::plain()),
            rec("• ", Face::plain()),
            rec("bravo", Face::plain()),
            rec(
                "cargo test",
                Face {
                    code: true,
                    ..Face::plain()
                },
            ),
            rec(
                "│ ",
                Face {
                    quoted: true,
                    ..Face::plain()
                },
            ),
            rec(
                "quoted line",
                Face {
                    quoted: true,
                    ..Face::plain()
                },
            ),
            rec("A ", Face::plain()),
            rec(
                "link",
                Face {
                    link: true,
                    ..Face::plain()
                },
            ),
            rec(" here.", Face::plain()),
            rec("A ", Face::plain()),
            rec(
                "struck",
                Face {
                    strikethrough: true,
                    ..Face::plain()
                },
            ),
            rec(" run.", Face::plain()),
            rec("│ ", Face::plain()),
            rec(
                "a",
                Face {
                    strong: true,
                    ..Face::plain()
                },
            ),
            rec(" │ ", Face::plain()),
            rec(
                "b",
                Face {
                    strong: true,
                    ..Face::plain()
                },
            ),
            rec(" │", Face::plain()),
            rec("├───┼───┤", Face::plain()),
            rec("│ 1 │ 2 │", Face::plain()),
            rec("[✓] ", Face::plain()),
            rec("done item", Face::plain()),
            rec("[ ] ", Face::plain()),
            rec("open item", Face::plain()),
            rec("VERIFY: this is prose, not a task", Face::plain()),
        ];

        // Unsuffixed, because `MDWIDTHS`' number scan is `\b(\d+)\b` and does
        // not see `58u16`; the script's own header says to write them bare.
        for width in [58, 78] {
            let out = lines(face_field_fixture(), width);
            let got: Vec<&Segment> = out.iter().flat_map(|l| l.segments.iter()).collect();
            assert_eq!(
                got.len(),
                recorded.len(),
                "width {width}: segment count moved"
            );
            for (segment, row) in got.iter().zip(recorded.iter()) {
                assert_eq!(segment.text, row.text, "width {width}: segment text moved");
                assert_eq!(segment.face, row.face, "width {width}: {:?}", row.text);

                // The two new fields, on every segment, for every construct —
                // and `spec-emphasis`'s own addition, `delta`, which this
                // fixture holds no `- **WHEN**` bullet to weaken: nothing
                // here is a scenario clause, so `label` stays `None` exactly
                // as before, and `delta` — never set by this module at
                // all — stays `None` beside it.
                assert!(
                    !segment.face.muted,
                    "width {width}: {:?} is muted",
                    row.text
                );
                assert_eq!(segment.face.label, None, "width {width}: {:?}", row.text);
                assert_eq!(segment.face.delta, None, "width {width}: {:?}", row.text);
            }

            // The `VERIFY:` paragraph in particular: one plain segment carrying
            // the whole sentence and no label at all.
            let verify = got
                .iter()
                .find(|s| s.text.starts_with("VERIFY:"))
                .expect("the VERIFY paragraph reached no segment");
            assert_eq!(verify.text, "VERIFY: this is prose, not a task");
            assert_eq!(verify.face, Face::plain());
            assert_eq!(verify.face.label, None);
            assert_eq!(verify.face.delta, None);
        }
    }

    /// `markdown-render` :: "Every segment `lines` returns carries no delta".
    #[test]
    fn every_segment_lines_returns_carries_no_delta() {
        let source = "# Heading\n\
                       \n\
                       A paragraph of prose.\n\
                       \n\
                       | a | b |\n\
                       |---|---|\n\
                       | 1 | 2 |\n\
                       \n\
                       ```sh\n\
                       cargo test\n\
                       ```\n\
                       \n\
                       > quoted line\n\
                       \n\
                       - [ ] an open task\n\
                       \n\
                       - **WHEN** the schema declares four artifacts\n";
        // Unsuffixed, because `MDWIDTHS`' number scan is `\b(\d+)\b` and does
        // not see `58u16`; the script's own header says to write them bare.
        for width in [58, 78] {
            let out = lines(source, width);
            let segments: Vec<&Segment> = out.iter().flat_map(|l| l.segments.iter()).collect();
            assert!(
                !segments.is_empty(),
                "width {width}: the fixture produced no segments"
            );
            for segment in &segments {
                assert_eq!(
                    segment.face.delta, None,
                    "width {width}: {:?} carries a delta",
                    segment.text
                );
            }
        }

        assert_eq!(
            Face::plain(),
            Face {
                delta: None,
                ..Face::plain()
            },
            "no markdown construct can set the badge field only ui::detail writes"
        );
    }

    /// `markdown-render` :: "A scenario's three clauses are coloured by
    /// position".
    #[test]
    fn a_scenarios_three_clauses_are_coloured_by_position() {
        let source = "- **WHEN** the schema declares four artifacts\n\
                       - **THEN** the tab bar shows four\n\
                       - **AND** the first is active\n";
        // Unsuffixed, because `MDWIDTHS`' number scan is `\b(\d+)\b` and does
        // not see `58u16`; the script's own header says to write them bare.
        for width in [58, 78] {
            let out = lines(source, width);
            let segments: Vec<&Segment> = out.iter().flat_map(|l| l.segments.iter()).collect();

            for (keyword, role, rest) in [
                (
                    "WHEN",
                    crate::tasks::LabelRole::Change,
                    " the schema declares four artifacts",
                ),
                (
                    "THEN",
                    crate::tasks::LabelRole::Confirm,
                    " the tab bar shows four",
                ),
                (
                    "AND",
                    crate::tasks::LabelRole::Confirm,
                    " the first is active",
                ),
            ] {
                let kw = segments
                    .iter()
                    .find(|s| s.text == keyword)
                    .unwrap_or_else(|| panic!("width {width}: {keyword:?} segment missing"));
                assert_eq!(kw.face.label, Some(role), "width {width}: {keyword:?}");
                // The colour is added beside the author's bold, never in
                // place of it.
                assert!(kw.face.strong, "width {width}: {keyword:?} lost its bold");

                let rest_seg = segments
                    .iter()
                    .find(|s| s.text == rest)
                    .unwrap_or_else(|| panic!("width {width}: {rest:?} segment missing"));
                assert_eq!(
                    rest_seg.face.label, None,
                    "width {width}: {rest:?} carries a label"
                );
            }
        }
    }

    /// `markdown-render` :: "`AND` inherits the clause above it and resets at
    /// a heading".
    #[test]
    fn and_inherits_the_clause_above_it_and_resets_at_a_heading() {
        let source = "#### Scenario: a\n\
                       \n\
                       - **WHEN** x\n\
                       - **AND** y\n\
                       \n\
                       #### Scenario: b\n\
                       \n\
                       - **AND** z\n";
        for width in [58, 78] {
            let out = lines(source, width);
            let segments: Vec<&Segment> = out.iter().flat_map(|l| l.segments.iter()).collect();
            let ands: Vec<&Segment> = segments
                .iter()
                .copied()
                .filter(|s| s.text == "AND")
                .collect();
            assert_eq!(ands.len(), 2, "width {width}: expected two AND segments");
            assert_eq!(
                ands[0].face.label,
                Some(crate::tasks::LabelRole::Change),
                "width {width}: the first AND inherits the WHEN above it"
            );
            assert_eq!(
                ands[1].face.label,
                Some(crate::tasks::LabelRole::Other),
                "width {width}: the heading resets the remembered position"
            );
        }
    }

    /// `markdown-render` :: "Only a run opening a list item is a keyword".
    #[test]
    fn only_a_run_opening_a_list_item_is_a_keyword() {
        let sources = [
            "**WHEN** not in a list",
            "- the **WHEN** clause is described",
            "- **Note** this is prose",
            "- **when** lowercase",
        ];
        for width in [58, 78] {
            for source in sources {
                let out = lines(source, width);
                let segments: Vec<&Segment> = out.iter().flat_map(|l| l.segments.iter()).collect();
                for segment in &segments {
                    assert_eq!(
                        segment.face.label, None,
                        "width {width}: {source:?} produced a label on {:?}",
                        segment.text
                    );
                }
                // Declining to classify changed nothing else: the bold runs
                // still carry `strong: true`.
                let bold: Vec<&&Segment> = segments
                    .iter()
                    .filter(|s| s.text.eq_ignore_ascii_case("when") || s.text == "Note")
                    .collect();
                assert!(
                    !bold.is_empty(),
                    "width {width}: {source:?} produced no bold run to check"
                );
                for segment in bold {
                    assert!(
                        segment.face.strong,
                        "width {width}: {source:?}: {:?} lost its bold",
                        segment.text
                    );
                }
            }
        }
    }

    #[test]
    fn no_line_exceeds_the_width_it_was_given() {
        let source = composite_fixture();
        // `responsive-layout`'s wide-character and ZWJ sources, checked at
        // the same widths as the ASCII composite fixture above — `columns`,
        // not `chars().count()`, is the measure a `chars().count()`-based
        // wrap would silently disagree with here, since these two sources
        // hold no ASCII at all.
        let cjk_500 = "日本語".repeat(84);
        let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}";
        let zwj_run = family.repeat(50);
        for width in [0u16, 1, 2, 3, 10, 58, 78, 200] {
            let out = lines(&source, width);
            for line in &out {
                assert!(
                    columns(&line.text()) <= width as usize,
                    "width {width}: line {:?} exceeds it",
                    line.text()
                );
            }
            for extra in [&cjk_500, &zwj_run] {
                let out = lines(extra, width);
                for line in &out {
                    assert!(
                        columns(&line.text()) <= width as usize,
                        "width {width}: CJK/ZWJ line {:?} exceeds it",
                        line.text()
                    );
                }
            }
        }
        for width in [58, 78] {
            let text = text_of(&lines(&source, width)).join("\n");
            assert!(text.contains("headingword"), "heading missing at {width}");
            assert!(
                text.contains("paragraphword"),
                "paragraph missing at {width}"
            );
            assert!(
                text.contains("bulletword"),
                "bullet item missing at {width}"
            );
            assert!(
                text.contains("nestedword"),
                "nested item missing at {width}"
            );
            assert!(text.contains("codeword"), "code missing at {width}");
            assert!(text.contains("quoteword"), "quote missing at {width}");
            assert!(text.contains("linkword"), "link text missing at {width}");
            assert!(text.contains("tableword"), "table missing at {width}");
            assert!(text.contains("struckword"), "struck run missing at {width}");
            assert!(
                text.contains("nestedquoteword"),
                "nested quote missing at {width}"
            );
            assert!(
                text.contains("checkedtaskword") && text.contains("uncheckedtaskword"),
                "task-list items missing at {width}"
            );
            assert!(
                text.contains("quotedtaskword"),
                "quote-nested task-list item missing at {width}"
            );
            assert!(
                !text.contains("design.md"),
                "link destination leaked at {width}"
            );
        }
    }

    /// `markdown-render` :: "Each emitted glyph measures one column, and the
    /// set is complete". The single property the whole glyph change rests on:
    /// were any of the seven two columns under `layout::columns` — the measure
    /// `Buffer::set_string` itself consumes — every line grammar built on them
    /// would overrun its region silently.
    ///
    /// The set-difference leg reads [`composite_fixture`], which deliberately
    /// holds no ordered list and no image: an ordered list renders `2.` from a
    /// source `1.` and an image renders `[img]`, each contributing a non-source
    /// character that is not one of these glyphs, which would make the equality
    /// unfalsifiable rather than stricter.
    #[test]
    fn every_emitted_glyph_measures_one_column() {
        let glyphs = ['•', '│', '─', '├', '┼', '┤', '✓'];
        for glyph in glyphs {
            assert_eq!(columns(&glyph.to_string()), 1, "{glyph:?}");
        }

        let source = composite_fixture();
        for width in [58, 78] {
            let rendered = text_of(&lines(&source, width));
            for text in &rendered {
                assert!(
                    columns(text) <= width as usize,
                    "width {width}: {text:?} exceeds it"
                );
            }
            let in_source: std::collections::BTreeSet<char> = source.chars().collect();
            let added: std::collections::BTreeSet<char> = rendered
                .join("")
                .chars()
                .filter(|c| !c.is_whitespace() && !in_source.contains(c))
                .collect();
            // Measured on what is actually emitted, not only on the declared
            // list above: a glyph added later without a width assertion is a
            // two-column character reaching the buffer, and this is the leg
            // that sees it.
            for c in &added {
                assert_eq!(columns(&c.to_string()), 1, "width {width}: emitted {c:?}");
            }
            assert_eq!(
                added,
                glyphs
                    .into_iter()
                    .collect::<std::collections::BTreeSet<_>>(),
                "width {width}: the rendering's non-source characters are not exactly the \
                 declared glyph set"
            );
        }
    }

    #[test]
    fn lines_is_total_over_arbitrary_input() {
        let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}";
        let pathological = [
            "#".to_string(),
            "```sh\nunclosed fence".to_string(),
            "[link(".to_string(),
            "x".repeat(500),
            "\n\n\n\n\n".to_string(),
            "     ".to_string(),
            "**bold".to_string(),
            "hold \u{0}my\u{301}beer".to_string(),
            // A 500-column CJK run with no space, and a run of family emoji
            // joined by zero-width joiners — `markdown-render`'s own named
            // sources for "Rendering is total over arbitrary input".
            "日本語".repeat(84),
            family.repeat(50),
            // `markdown-render`'s five adversarial table sources: a header
            // with no delimiter row (so it is not a table at all), a body
            // row carrying more cells than the header declares, one
            // carrying fewer, a table of forty columns — `4n + 1` is 161,
            // so every swept width takes the narrow fallback — and a table
            // whose single cell is a 500-column CJK run with no space.
            "| a | b |\n".to_string(),
            "| a | b |\n|---|---|\n| c | d | e | f |\n".to_string(),
            "| a | b | c |\n|---|---|---|\n| d |\n".to_string(),
            format!("|{}\n|{}\n", " x |".repeat(40), "---|".repeat(40)),
            format!("| h |\n|---|\n| {} |\n", "日本語".repeat(84)),
            // `markdown-render`'s three adversarial task-list sources: an
            // item whose text is a 500-character token with no space, a bare
            // marker with no text after it, and one nested three quote levels
            // deep — each reaching the four-column prefix's own
            // `saturating_sub` at widths 1 and 2.
            format!("- [ ] {}\n", "x".repeat(500)),
            "- [x]\n".to_string(),
            "> > > - [x] deep\n".to_string(),
        ];
        for width in [1u16, 2, 58, 78] {
            for source in &pathological {
                let out = lines(source, width);
                for line in &out {
                    assert!(
                        columns(&line.text()) <= width as usize,
                        "width {width} source {source:?}: line {:?} exceeds it",
                        line.text()
                    );
                }
            }
            let hard = lines(&"x".repeat(500), width);
            assert!(hard.iter().all(|l| columns(&l.text()) <= width as usize));
            assert!(
                hard.iter().any(|l| columns(&l.text()) == width as usize),
                "the 500-character token must be hard-split into full-width lines at {width}"
            );
        }

        // At width 1, a two-column cluster — a CJK ideograph or a
        // zero-width-joined family emoji — has no prefix that fits: it is
        // dropped, producing an empty line for that position rather than a
        // line that measures 2.
        let cjk_500 = "日本語".repeat(84);
        let zwj_run = family.repeat(50);
        for source in [&cjk_500, &zwj_run] {
            let out = lines(source, 1);
            assert!(
                out.iter().all(|l| columns(&l.text()) <= 1),
                "width 1: {source:?} produced an over-wide line"
            );
            assert!(
                out.iter().any(|l| l.text().is_empty()),
                "width 1: a two-column cluster must be dropped, producing an empty line, \
                 not a two-column one: {:?}",
                out.iter().map(|l| l.text()).collect::<Vec<_>>()
            );
        }
    }

    /// `markdown-render` :: "A wide-character document wraps by columns at
    /// both mandated widths". The paragraph is one unbroken 120-character,
    /// 240-column CJK run — no interior space — so it exercises the
    /// oversized-token split inside `wrap_prose`, not the ordinary
    /// greedy-wrap path. Every character of "日本語" is uniformly 3 bytes
    /// and 2 columns, which is what makes the byte-offset table below
    /// exact rather than approximate: it states the expected re-slice by
    /// hand instead of recomputing it from the function under test, so a
    /// regression to `chars().count()` — which would chunk by *character*
    /// count, not column count, and so would land on entirely different
    /// byte offsets — is caught rather than silently reproduced.
    #[test]
    fn a_wide_character_document_wraps_by_columns_at_both_mandated_widths() {
        let paragraph = "日本語".repeat(40);
        let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}";
        let source = format!("{paragraph}\n\n- \u{1F389} celebrate\n- {family} family\n");

        for width in [58, 78] {
            let out = lines(&source, width);
            for line in &out {
                assert!(
                    columns(&line.text()) <= width as usize,
                    "width {width}: {:?} exceeds it",
                    line.text()
                );
            }
            assert!(
                out.iter().any(|l| {
                    let c = columns(&l.text());
                    c == width as usize || c + 1 == width as usize
                }),
                "width {width}: no line fills the region"
            );
        }

        // At 58, the wide paragraph must wrap into strictly more lines than
        // the same paragraph with every "日本語" replaced by three ASCII
        // letters: the wide form consumes twice the columns per character.
        let ascii_paragraph = paragraph.replace("日本語", "abc");
        let wide_lines = lines(&paragraph, 58).len();
        let ascii_lines = lines(&ascii_paragraph, 58).len();
        assert!(
            wide_lines > ascii_lines,
            "width 58: wide paragraph produced {wide_lines} lines, ascii {ascii_lines}"
        );

        // The oversized-token split cuts the unbroken paragraph at exact,
        // hand-computed byte offsets: a `width`-column chunk is
        // `width / 2` characters, `3 * (width / 2)` bytes, since every
        // character here is uniformly 2 columns and 3 bytes.
        let texts58: Vec<String> = lines(&paragraph, 58).iter().map(Line::text).collect();
        let expected_bytes_58 = [87usize, 87, 87, 87, 12];
        assert_eq!(
            texts58.len(),
            expected_bytes_58.len(),
            "width 58: {texts58:?}"
        );
        let mut offset = 0usize;
        for (text, &blen) in texts58.iter().zip(expected_bytes_58.iter()) {
            assert_eq!(text.len(), blen, "width 58: line byte length");
            assert_eq!(
                text.as_str(),
                &paragraph[offset..offset + blen],
                "width 58: rendered line must equal the source re-sliced at this exact byte \
                 offset"
            );
            offset += blen;
        }
        assert_eq!(
            offset,
            paragraph.len(),
            "width 58: every byte accounted for"
        );

        let texts78: Vec<String> = lines(&paragraph, 78).iter().map(Line::text).collect();
        let expected_bytes_78 = [117usize, 117, 117, 9];
        assert_eq!(
            texts78.len(),
            expected_bytes_78.len(),
            "width 78: {texts78:?}"
        );
        let mut offset = 0usize;
        for (text, &blen) in texts78.iter().zip(expected_bytes_78.iter()) {
            assert_eq!(text.len(), blen, "width 78: line byte length");
            assert_eq!(
                text.as_str(),
                &paragraph[offset..offset + blen],
                "width 78: rendered line must equal the source re-sliced at this exact byte \
                 offset"
            );
            offset += blen;
        }
        assert_eq!(
            offset,
            paragraph.len(),
            "width 78: every byte accounted for"
        );
    }

    #[test]
    fn the_six_heading_levels_carry_their_markers() {
        let source = "# One\n\n## Two\n\n### Three\n\n#### Four\n\n##### Five\n\n###### Six\n";
        for width in [58, 78] {
            let out = lines(source, width);
            let non_blank: Vec<&Line> = out.iter().filter(|l| !l.segments.is_empty()).collect();
            let texts: Vec<String> = non_blank.iter().map(|l| l.text()).collect();
            assert_eq!(
                texts,
                vec![
                    "# One",
                    "## Two",
                    "### Three",
                    "#### Four",
                    "##### Five",
                    "###### Six"
                ]
            );
            for (i, line) in non_blank.iter().enumerate() {
                let level = (i + 1) as u8;
                for seg in &line.segments {
                    assert_eq!(seg.face.heading, Some(level));
                    assert!(!seg.face.strong);
                    assert!(!seg.face.emphasis);
                    assert!(!seg.face.code);
                    assert!(!seg.face.link);
                    assert!(!seg.face.quoted);
                }
            }
        }
    }

    #[test]
    fn a_long_heading_wraps_under_its_text_column() {
        let source = "## alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima mike november\n";
        for width in [58, 78] {
            let out = lines(source, width);
            let non_blank: Vec<&Line> = out.iter().filter(|l| !l.segments.is_empty()).collect();
            assert!(non_blank[0].text().starts_with("## alpha"));
            for line in &non_blank[1..] {
                let text = line.text();
                assert!(text.starts_with("   "), "width {width}: {text:?}");
                assert_ne!(text.chars().nth(3), Some(' '));
            }
            for line in &non_blank {
                for seg in &line.segments {
                    assert_eq!(seg.face.heading, Some(2));
                }
            }
        }
    }

    #[test]
    fn inline_constructs_become_faced_segments() {
        let source =
            "This **is** a *very* nice `example()` call for [the design](design.md) doc.\n";
        for width in [58, 78] {
            let out = lines(source, width);
            let all_segments: Vec<&Segment> = out.iter().flat_map(|l| l.segments.iter()).collect();

            let is = all_segments.iter().find(|s| s.text == "is").unwrap();
            assert!(is.face.strong);
            assert!(!is.face.emphasis && !is.face.code && !is.face.link && !is.face.quoted);

            let very = all_segments.iter().find(|s| s.text == "very").unwrap();
            assert!(very.face.emphasis);
            assert!(!very.face.strong);

            let code = all_segments.iter().find(|s| s.text == "example()").unwrap();
            assert!(code.face.code);

            let link = all_segments
                .iter()
                .find(|s| s.text == "the design")
                .unwrap();
            assert!(link.face.link);

            assert!(!all_segments.iter().any(|s| s.text.contains("design.md")));

            let joined = text_of(&out).join(" ");
            assert!(joined.contains("This is a very nice example() call for the design doc."));
        }
    }

    #[test]
    fn nested_emphasis_composes() {
        let source = "***both*** and [**bold link**](x)\n";
        for width in [58, 78] {
            let out = lines(source, width);
            let all_segments: Vec<&Segment> = out.iter().flat_map(|l| l.segments.iter()).collect();
            let both = all_segments.iter().find(|s| s.text == "both").unwrap();
            assert!(both.face.strong && both.face.emphasis);
            let bold_link = all_segments.iter().find(|s| s.text == "bold link").unwrap();
            assert!(bold_link.face.link && bold_link.face.strong);
        }
    }

    #[test]
    fn an_image_renders_its_alt_text() {
        let source = "Before ![a diagram](x.png) and ![](y.png) after\n";
        for width in [58, 78] {
            let out = lines(source, width);
            let all_segments: Vec<&Segment> = out.iter().flat_map(|l| l.segments.iter()).collect();
            let diagram = all_segments.iter().find(|s| s.text == "a diagram").unwrap();
            assert!(diagram.face.link);
            let img = all_segments.iter().find(|s| s.text == "[img]").unwrap();
            assert!(img.face.link);
            assert!(!all_segments.iter().any(|s| s.text.contains("x.png")));
            assert!(!all_segments.iter().any(|s| s.text.contains("y.png")));
            let joined = text_of(&out).join(" ");
            assert!(joined.contains("a diagram"));
            assert!(joined.contains("[img]"));
        }
    }

    #[test]
    fn a_faced_run_split_across_a_wrap_keeps_its_face() {
        let source = "**alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima mike november oscar papa**\n";
        for width in [58, 78] {
            let out = lines(source, width);
            let non_blank: Vec<&Line> = out.iter().filter(|l| !l.segments.is_empty()).collect();
            assert_eq!(non_blank.len(), 2, "width {width}");
            for line in &non_blank {
                for seg in &line.segments {
                    assert!(seg.face.strong, "width {width}: {seg:?} not strong");
                }
            }
        }
        let expected58 = vec![
            "alpha bravo charlie delta echo foxtrot golf hotel india".to_string(),
            "juliett kilo lima mike november oscar papa".to_string(),
        ];
        assert_eq!(
            text_of(&lines(source, 58))
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>(),
            expected58
        );
        // Both widths' exact strings must match paragraph_wraps_at_58_and_78's,
        // per the spec scenario's "the same two strings the paragraph-wrap
        // scenario names" clause — checking only 58 would miss a bug specific
        // to the 78-width wrap point.
        let expected78 = vec![
            "alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima mike"
                .to_string(),
            "november oscar papa".to_string(),
        ];
        assert_eq!(
            text_of(&lines(source, 78))
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>(),
            expected78
        );
    }

    #[test]
    fn a_soft_break_starts_a_new_line() {
        // The scenario's name is its delta merge key and is kept verbatim;
        // its assertions are the inverse of the ones it carried, which is
        // what proves the reversal landed.
        let sixty_words: String = {
            let words: Vec<String> = (0..60).map(|i| format!("word{i:02}")).collect();
            format!("{}\n", words.join("\n"))
        };
        for width in [58, 78] {
            let folded = text_of(&lines("first line\nsecond line\n", width));
            assert_eq!(folded, vec!["first line second line"], "width {width}");

            // Two trailing spaces are a hard break: the author's explicit
            // break survives where the incidental one does not.
            let hard = text_of(&lines("first line  \nsecond line\n", width));
            assert_eq!(hard, vec!["first line", "second line"], "width {width}");

            // Sixty words, one per source line, reflow to fill the region.
            let reflowed = text_of(&lines(&sixty_words, width));
            assert!(reflowed.len() < 60, "width {width}: {}", reflowed.len());
            let (last, filled) = reflowed.split_last().expect("at least one line");
            for line in filled {
                // Within one word of the width: every line but the last is
                // filled, rather than the source's one word per line.
                assert!(
                    columns(line) + 8 > width as usize,
                    "width {width}: short line {line:?}"
                );
                assert!(columns(line) <= width as usize, "width {width}: {line:?}");
            }
            assert!(columns(last) <= width as usize, "width {width}: {last:?}");

            // A fenced block is verbatim: the fold must not leak into it.
            // The trailing blank the verbatim path emits for the source's
            // final newline is pre-existing and not this rule's subject.
            let fenced = text_of(&lines("```\nalpha\nbravo\n```\n", width));
            let fenced: Vec<&String> = fenced.iter().filter(|s| !s.is_empty()).collect();
            assert_eq!(fenced, vec!["alpha", "bravo"], "width {width}");
        }
        assert!(
            text_of(&lines(&sixty_words, 78)).len() < text_of(&lines(&sixty_words, 58)).len(),
            "the reflow is width-driven"
        );
    }

    #[test]
    fn exactly_one_blank_line_separates_blocks() {
        let source = "# One\n\nAlpha bravo charlie.\n\n\n\n## Two\n\nDelta echo foxtrot.\n";
        for width in [58, 78] {
            let out = lines(source, width);
            assert!(!out.first().unwrap().segments.is_empty(), "width {width}");
            assert!(!out.last().unwrap().segments.is_empty(), "width {width}");
            for pair in out.windows(2) {
                assert!(
                    !(pair[0].segments.is_empty() && pair[1].segments.is_empty()),
                    "width {width}: two consecutive blank lines"
                );
            }
            let blank_run_lengths: Vec<usize> = {
                let mut lens = Vec::new();
                let mut run = 0usize;
                for line in &out {
                    if line.segments.is_empty() {
                        run += 1;
                    } else if run > 0 {
                        lens.push(run);
                        run = 0;
                    }
                }
                lens
            };
            assert!(
                blank_run_lengths.iter().all(|&n| n == 1),
                "width {width}: {blank_run_lengths:?}"
            );
            assert_eq!(blank_run_lengths.len(), 3, "width {width}");
        }
    }

    #[test]
    fn bullet_items_carry_their_marker_and_hanging_indent() {
        let source = "- alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima mike november oscar papa\n";
        for width in [58, 78] {
            let out = lines(source, width);
            let non_blank: Vec<&Line> = out.iter().filter(|l| !l.segments.is_empty()).collect();
            assert!(non_blank[0].text().starts_with("• alpha"), "width {width}");
            for line in &non_blank[1..] {
                let text = line.text();
                assert!(text.starts_with("  "), "width {width}: {text:?}");
                assert_ne!(text.chars().nth(2), Some(' '), "width {width}: {text:?}");
            }
            for line in &non_blank {
                for seg in &line.segments {
                    assert_eq!(seg.face, Face::plain());
                }
            }
        }
        let at58 = lines(source, 58);
        let first58 = at58[0].text();
        assert_eq!(
            first58,
            "• alpha bravo charlie delta echo foxtrot golf hotel india"
        );
        assert_eq!(columns(&first58), 57);
        let at78 = lines(source, 78);
        let first78 = at78[0].text();
        assert_eq!(
            first78,
            "• alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima"
        );
        assert_eq!(columns(&first78), 75);
    }

    #[test]
    fn an_ordered_list_numbers_from_its_start_value() {
        let source = "7. seven\n8. eight\n9. nine\n";
        for width in [58, 78] {
            let texts: Vec<String> = text_of(&lines(source, width))
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect();
            assert_eq!(
                texts,
                vec!["7. seven", "8. eight", "9. nine"],
                "width {width}"
            );
        }
        let restart = "1. one\n1. one again\n";
        for width in [58, 78] {
            let texts: Vec<String> = text_of(&lines(restart, width))
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect();
            assert_eq!(texts, vec!["1. one", "2. one again"], "width {width}");
        }
    }

    #[test]
    fn a_nested_list_indents_two_columns_per_level() {
        let source = "- outer\n  - inner\n    - deepest\n";
        for width in [58, 78] {
            let texts: Vec<String> = text_of(&lines(source, width))
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect();
            assert_eq!(
                texts,
                vec!["• outer", "  • inner", "    • deepest"],
                "width {width}"
            );
        }

        let wrapped = "- outer\n  - inner alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima mike november\n";
        for width in [58, 78] {
            let out = lines(wrapped, width);
            let non_blank: Vec<&Line> = out.iter().filter(|l| !l.segments.is_empty()).collect();
            let inner_start = non_blank
                .iter()
                .position(|l| l.text().starts_with("  • inner"))
                .expect("the inner item's first line");
            for line in &non_blank[inner_start + 1..] {
                let text = line.text();
                assert!(text.starts_with("    "), "width {width}: {text:?}");
                assert_ne!(text.chars().nth(4), Some(' '), "width {width}: {text:?}");
            }
        }
    }

    #[test]
    fn a_fenced_code_block_is_verbatim() {
        let source = "```sh\ncargo test --all-features\n  indented\n```\n";
        for width in [58, 78] {
            let out = lines(source, width);
            let code_lines: Vec<&Line> = out.iter().filter(|l| !l.segments.is_empty()).collect();
            assert_eq!(code_lines.len(), 2, "width {width}");
            assert_eq!(code_lines[0].text(), "cargo test --all-features");
            assert_eq!(code_lines[1].text(), "  indented");
            for line in &code_lines {
                for seg in &line.segments {
                    assert!(seg.face.code, "width {width}");
                    assert!(
                        !seg.face.strong
                            && !seg.face.emphasis
                            && !seg.face.link
                            && !seg.face.quoted
                    );
                }
            }
            let joined = text_of(&out).join("\n");
            assert!(!joined.contains("```"), "width {width}");
            assert!(!joined.contains("sh"), "width {width}");
        }
    }

    #[test]
    fn a_long_code_line_is_hard_split() {
        let source = format!("```\n{}\n```\n", "x".repeat(130));
        let at58: Vec<String> = text_of(&lines(&source, 58))
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(
            at58.iter().map(|s| columns(s)).collect::<Vec<_>>(),
            vec![58, 58, 14]
        );
        assert_eq!(at58.concat(), "x".repeat(130));

        let at78: Vec<String> = text_of(&lines(&source, 78))
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(
            at78.iter().map(|s| columns(s)).collect::<Vec<_>>(),
            vec![78, 52]
        );
        assert_eq!(at78.concat(), "x".repeat(130));
    }

    #[test]
    fn an_indented_code_block_matches_the_fenced_form() {
        let fenced = "```\nalpha bravo\ncharlie\n```\n";
        let indented = "    alpha bravo\n    charlie\n";
        for width in [58, 78] {
            let fenced_texts: Vec<String> = text_of(&lines(fenced, width))
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect();
            let indented_texts: Vec<String> = text_of(&lines(indented, width))
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect();
            assert_eq!(fenced_texts, indented_texts, "width {width}");
            for line in lines(indented, width)
                .iter()
                .filter(|l| !l.segments.is_empty())
            {
                for seg in &line.segments {
                    assert!(seg.face.code, "width {width}");
                }
            }
        }
    }

    #[test]
    fn a_raw_html_block_renders_verbatim() {
        let source = "<details><summary>Notes</summary>\n\nAfter.\n";
        for width in [58, 78] {
            let out = lines(source, width);
            let html_line = out
                .iter()
                .find(|l| l.text() == "<details><summary>Notes</summary>")
                .unwrap_or_else(|| panic!("width {width}: html line missing"));
            for seg in &html_line.segments {
                assert!(seg.face.code, "width {width}");
            }
            let joined = text_of(&out).join("\n");
            assert!(joined.contains("<details>"), "width {width}");
        }
    }

    #[test]
    fn a_block_quote_prefixes_every_line() {
        let source = "> alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima mike november oscar papa\n";
        for width in [58, 78] {
            let out = lines(source, width);
            let non_blank: Vec<&Line> = out.iter().filter(|l| !l.segments.is_empty()).collect();
            for line in &non_blank {
                let text = line.text();
                assert!(text.starts_with("│ "), "width {width}: {text:?}");
                for seg in &line.segments {
                    assert!(seg.face.quoted, "width {width}: {seg:?}");
                }
            }
        }
        let at58 = lines(source, 58);
        let first58 = at58.iter().find(|l| !l.segments.is_empty()).unwrap().text();
        assert_eq!(
            first58,
            "│ alpha bravo charlie delta echo foxtrot golf hotel india"
        );
        assert_eq!(columns(&first58), 57);

        let nested = "> > nested alpha bravo\n";
        for width in [58, 78] {
            let out = lines(nested, width);
            let first = out.iter().find(|l| !l.segments.is_empty()).unwrap().text();
            assert!(first.starts_with("│ │ "), "width {width}: {first:?}");
        }
    }

    #[test]
    fn a_thematic_break_fills_the_width() {
        let source = "alpha before\n\n---\n\nbravo after\n";
        for width in [58, 78] {
            let out = lines(source, width);
            let idx = out
                .iter()
                .position(|l| l.segments.len() == 1 && l.segments[0].text.chars().all(|c| c == '─'))
                .unwrap_or_else(|| panic!("width {width}: no rule line found"));
            let rule = &out[idx];
            assert_eq!(columns(&rule.text()), width as usize);
            assert_eq!(rule.segments[0].face, Face::plain());
            assert!(out[idx - 1].segments.is_empty(), "width {width}");
            assert!(out[idx + 1].segments.is_empty(), "width {width}");
        }
    }

    /// `degraded-coverage` :: "A footnote, strikethrough, and a table each render as literal
    /// source" — row 12. The scenario's name is kept verbatim because a delta's scenario
    /// headers are its merge key and OpenSpec has no scenario-level rename; its subject
    /// narrows again, to the **one** construct that remains unmodelled — a footnote
    /// reference and its definition, on a tab this test never marks tracked, so
    /// `ui::markdown::lines` (not `ui::tasks::lines`) is what renders it: rendered line
    /// count equals source line count, one line per source line, at both mandated markdown
    /// widths. The three that have left the set — a task-list item, a table, and a struck
    /// run — appear as the **discriminating controls**, so this test fails if the narrowing
    /// is not real.
    #[test]
    fn unmodelled_constructs_render_as_source() {
        let sources = [(
            "footnote reference and definition",
            "See it here[^1].\n\n[^1]: The note.\n",
        )];
        for width in [58, 78] {
            for (label, source) in sources {
                let rendered = lines(source, width);
                let source_lines: Vec<&str> = source.lines().collect();
                assert_eq!(
                    rendered.len(),
                    source_lines.len(),
                    "width {width}, {label}: rendered {} lines for {} source lines: {:?}",
                    rendered.len(),
                    source_lines.len(),
                    text_of(&rendered)
                );
                // The rendered text, not merely its LINE COUNT, must equal the source: a
                // renderer emitting the right number of blank lines would satisfy the count
                // assertion above and prove nothing (`degraded-states`' own Change Review
                // caught exactly this — a check that cannot fail is this project's own
                // defect class). Every line's text is compared byte-for-byte, in order, and
                // every line must carry SOME non-blank content (a construct rendered as
                // whitespace would still pass a bare inequality check).
                for (i, (rendered_line, source_line)) in
                    rendered.iter().zip(source_lines.iter()).enumerate()
                {
                    let text = rendered_line.text();
                    assert_eq!(
                        text.trim_end(),
                        *source_line,
                        "width {width}, {label}, source line {i}: rendered text does not \
                         match the source line verbatim"
                    );
                    assert!(
                        !text.trim().is_empty() || source_line.trim().is_empty(),
                        "width {width}, {label}, source line {i}: rendered as blank, not the \
                         source line's own text"
                    );
                    for seg in &rendered_line.segments {
                        assert_eq!(seg.face, Face::plain(), "width {width}, {label}");
                    }
                }
            }
        }

        // The first discriminating control: the task-list item, which has just
        // LEFT this row. The checkbox glyph stands in place of the bullet
        // marker and the literal `[ ]`/`[x]` source text is gone, so this
        // scenario fails if the narrowing is not real.
        let task_list_source = "- [ ] an item\n- [x] a done item\n";
        for width in [58, 78] {
            assert_eq!(
                non_blank(&lines(task_list_source, width)),
                vec!["[ ] an item".to_string(), "[✓] a done item".to_string()],
                "width {width}"
            );
        }

        // The hanging indent is derived from the marker actually rendered, not
        // from the bullet marker the item would have carried: `start_item`
        // builds `cont_prefix` before `TaskListMarker` is seen, and an arm that
        // rewrites only the first line's prefix leaves continuations two
        // columns in. No width or totality assertion can see that — a shorter
        // indent never overruns — so it is asserted directly here.
        let wrapping = "- [ ] alpha bravo charlie delta echo foxtrot golf hotel india \
                        juliett kilo lima mike november oscar papa\n";
        for width in [58, 78] {
            let item = non_blank(&lines(wrapping, width));
            assert!(item.len() > 1, "width {width}: the item must wrap");
            assert!(item[0].starts_with("[ ] alpha"), "width {width}: {item:?}");
            for text in &item[1..] {
                assert!(
                    text.starts_with("    ") && !text.starts_with("     "),
                    "width {width}: continuation {text:?} is not four columns in"
                );
            }
        }

        // The tracked-tasks carve-out this row's own wording names ("on a tab **other**
        // than the tracked-tasks one"): the SAME task-list source, dispatched through
        // `ui::tasks::lines` (what `ui::detail::content_lines` reaches for a tracked tab)
        // instead of `ui::markdown::lines`, renders the checklist grammar — a progress bar
        // above the items — rather than the markdown path's bare item lines. Both paths'
        // item glyph is the SAME `[✓]`, asserted here rather than assumed: it is the
        // structural answer to the drift `markdown-render` names.
        let progress = crate::tasks::Progress {
            completed: 1,
            total: 2,
        };
        for width in [58, 78] {
            let tracked = crate::ui::tasks::lines(task_list_source, &progress, width);
            let tracked_text: Vec<String> = tracked
                .iter()
                .map(|l| l.text().trim_end().to_string())
                .collect();
            assert!(
                tracked_text.iter().any(|l| l == "[✓] a done item"),
                "width {width}: the two renderers must agree on the item glyph: \
                 {tracked_text:?}"
            );
            assert_ne!(
                tracked_text,
                text_of(&lines(task_list_source, width)),
                "width {width}: the tracked tab must differ from the markdown rendering, or \
                 the carve-out proves nothing"
            );
        }

        // The remaining discriminating controls: the two constructs that left
        // this row earlier. A GFM table and a `~~struck~~` span in the same
        // fixture render as aligned columns and as a struck face rather than as
        // literal text, so this test fails if that narrowing is a reword that
        // changed nothing.
        let departed =
            "| Gate | Runner |\n|---|---|\n| Format | cargo fmt |\n\nA ~~struck~~ word.\n";
        for width in [58, 78] {
            let rendered = lines(departed, width);
            let texts = text_of(&rendered);
            assert!(
                texts.iter().any(|t| t == "│ Gate   │ Runner    │"),
                "width {width}: the table must render as aligned columns, not literal \
                 source: {texts:?}"
            );
            assert!(
                rendered
                    .iter()
                    .flat_map(|l| &l.segments)
                    .any(|s| s.text == "struck" && s.face.strikethrough),
                "width {width}: the struck run must carry the face, not render literally"
            );
            assert!(
                !texts.iter().any(|t| t.contains("~~")),
                "width {width}: the strikethrough markers must be consumed: {texts:?}"
            );
        }
    }

    /// The non-blank rendered lines' `text()` values — a table block emits no
    /// blank line of its own, so for a table-only source this is the whole
    /// rendering.
    fn non_blank(rendered: &[Line]) -> Vec<String> {
        text_of(rendered)
            .into_iter()
            .filter(|t| !t.is_empty())
            .collect()
    }

    /// The column widths a rendered table's delimiter line reports: each run of
    /// `─` between two junctions measures `w[j] + 2`. Read back out of the
    /// output rather than recomputed by the test, so an assertion on it is an
    /// assertion on what the reader is shown.
    fn allocated_widths(delimiter: &str) -> Vec<usize> {
        delimiter
            .trim_matches(|c| c == '├' || c == '┤')
            .split('┼')
            .map(|run| columns(run) - 2)
            .collect()
    }

    /// Column `j`'s own field of a rendered row line: the `w[j]` columns between
    /// that column's two padding spaces. The leading `│` costs one column and
    /// every earlier column costs `w[i] + 3` — a padding space, its content, a
    /// padding space, and the `│` that closes it.
    fn field(row: &str, w: &[usize], j: usize) -> String {
        let start: usize = 1 + w[..j].iter().map(|x| x + 3).sum::<usize>() + 1;
        row.chars().skip(start).take(w[j]).collect()
    }

    /// `markdown-render` :: "A table that fits renders as aligned columns at both
    /// mandated widths". The literals below are the allocation rule's own output
    /// and are derived, not chosen: `nat = [6, 12]` (`Format` and `cargo clippy`
    /// are the widest cells), the table fits at both 58 and 78, so `w = nat` and
    /// `total = 3n + 1 + sum(w) = 7 + 18 = 25`.
    #[test]
    fn a_table_that_fits_renders_as_aligned_columns() {
        let source =
            "| Gate | Runner |\n|---|---|\n| Format | cargo fmt |\n| Lint | cargo clippy |\n";
        for width in [58, 78] {
            let rendered = lines(source, width);
            let texts = non_blank(&rendered);
            assert_eq!(
                texts,
                vec![
                    "│ Gate   │ Runner       │".to_string(),
                    "├────────┼──────────────┤".to_string(),
                    "│ Format │ cargo fmt    │".to_string(),
                    "│ Lint   │ cargo clippy │".to_string(),
                ],
                "width {width}"
            );
            for text in &texts {
                assert_eq!(columns(text), 25, "width {width}: {text:?}");
                assert!(
                    columns(text) < width as usize,
                    "width {width}: the table is stretched to the region rather than sized \
                     to its content"
                );
            }
            assert_eq!(allocated_widths(&texts[1]), vec![6, 12], "width {width}");

            // The header cells read bold through the existing `Strong` role;
            // every pipe, padding space, and delimiter-line segment is plain.
            let header = &rendered[0];
            let labels: Vec<&str> = header
                .segments
                .iter()
                .filter(|s| s.face.strong)
                .map(|s| s.text.as_str())
                .collect();
            assert_eq!(labels, vec!["Gate", "Runner"], "width {width}");
            for seg in &header.segments {
                if seg.face.strong {
                    assert_eq!(
                        seg.face,
                        Face {
                            strong: true,
                            ..Face::plain()
                        },
                        "width {width}: a header cell carries a face beyond `strong`"
                    );
                } else {
                    assert_eq!(seg.face, Face::plain(), "width {width}");
                }
            }
            for line in &rendered[1..] {
                for seg in &line.segments {
                    assert_eq!(
                        seg.face,
                        Face::plain(),
                        "width {width}: a non-header line carries a face"
                    );
                }
            }

            // The columns are aligned: the delimiter line's `├`/`┼`/`┤`
            // offsets are every row line's `│` offsets.
            let seps = |text: &str, set: &[char]| -> Vec<usize> {
                text.chars()
                    .enumerate()
                    .filter(|(_, c)| set.contains(c))
                    .map(|(i, _)| i)
                    .collect()
            };
            let expected = seps(&texts[1], &['├', '┼', '┤']);
            assert_eq!(expected.len(), 3, "width {width}: n + 1 separators");
            for (i, text) in texts.iter().enumerate() {
                if i == 1 {
                    continue;
                }
                let got = seps(text, &['│']);
                assert_eq!(got, expected, "width {width}: {text:?} is not aligned");
            }
        }
    }

    /// `markdown-render` :: "A cell wider than its column wraps rather than being
    /// truncated". `nat = [3, 98]` and the table fits at neither width, so the
    /// max-min rule caps the wide column: `w = [3, 48]` at 58 and `[3, 68]` at
    /// 78, and the 96-column cell wraps at both.
    #[test]
    fn a_wide_cell_wraps_within_its_column() {
        let cell_text = "alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo \
                         lima mike november oscar papa";
        assert_eq!(columns(cell_text), 98);
        let source = format!("| key | value |\n|---|---|\n| k | {cell_text} |\n");
        for (width, expect_w) in [(58, vec![3usize, 48]), (78, vec![3, 68])] {
            let rendered = lines(&source, width);
            let texts = non_blank(&rendered);
            let w = allocated_widths(&texts[1]);
            assert_eq!(w, expect_w, "width {width}");
            let total = 3 * w.len() + 1 + w.iter().sum::<usize>();
            assert_eq!(total, width as usize, "width {width}");

            let body = &texts[2..];
            assert!(
                body.len() > 1,
                "width {width}: the row must occupy more than one line"
            );
            for text in &texts {
                assert_eq!(columns(text), total, "width {width}: {text:?}");
                assert!(columns(text) <= width as usize, "width {width}");
            }

            // Every word of the cell is present, in order, with the single
            // spaces the wrap consumed restored.
            let rejoined = body
                .iter()
                .map(|l| field(l, &w, 1).trim_end().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            assert_eq!(rejoined, cell_text, "width {width}: the cell was truncated");

            // The first column's cell sits on the row's first line; its
            // continuation lines carry `w[0]` spaces in that column instead, so
            // the wrapped cell's lines align under it rather than under the row.
            assert_eq!(field(&body[0], &w, 0), "k  ", "width {width}");
            for text in &body[1..] {
                assert_eq!(
                    field(text, &w, 0),
                    " ".repeat(w[0]),
                    "width {width}: a continuation line repeats the first column"
                );
            }
        }
    }

    /// `markdown-render` :: "A table too wide for the region spends its columns on
    /// the narrow ones". `nat = [4, 4, 4, 60]`, `n = 4`, overhead `3n + 1 = 13`.
    /// At 58 `avail = 45` and the largest cap that fits is 33, so `w =
    /// [4, 4, 4, 33]`; at 78 `avail = 65` and the cap is 53. A proportional
    /// shrink would squeeze the three four-column labels instead, which is the
    /// alternative this rule exists to reject.
    #[test]
    fn a_wide_table_allocates_max_min_fairly() {
        let wide = "alpha bravo charlie delta echo foxtrot golf hotel india kilo";
        assert_eq!(columns(wide), 60);
        let source = format!(
            "| aaaa | bbbb | cccc | dddd |\n|---|---|---|---|\n| aaaa | bbbb | cccc | {wide} |\n"
        );
        for (width, expect_w) in [(58, vec![4usize, 4, 4, 33]), (78, vec![4, 4, 4, 53])] {
            let rendered = lines(&source, width);
            let texts = non_blank(&rendered);
            let w = allocated_widths(&texts[1]);
            assert_eq!(
                w, expect_w,
                "width {width}: the short columns must keep their natural width"
            );
            assert!(w.iter().all(|x| *x >= 1), "width {width}");
            let total = 3 * w.len() + 1 + w.iter().sum::<usize>();
            assert_eq!(total, width as usize, "width {width}");
            for text in &texts {
                assert_eq!(columns(text), total, "width {width}: {text:?}");
            }

            // Nothing was truncated to make it fit: every word of the wide cell
            // is somewhere in the rendering.
            let joined = texts.join("\n");
            for word in wide.split(' ') {
                assert!(
                    joined.contains(word),
                    "width {width}: {word:?} was truncated away"
                );
            }
        }
    }

    /// `markdown-render` :: "Alignment markers pad the cell on the side they
    /// name". The header cells are four columns wide precisely so `nat[j] = 4`
    /// and the padding is observable at all: a one-column-wide fixture renders
    /// left, centre, and right byte-identically and would pass against an
    /// implementation that ignores `Alignment` entirely.
    #[test]
    fn alignment_markers_pad_the_side_they_name() {
        let source = "| left | cent | rght |\n|:---|:--:|---:|\n| x | x | x |\n";
        for width in [58, 78] {
            let texts = non_blank(&lines(source, width));
            let w = allocated_widths(&texts[1]);
            assert_eq!(w, vec![4, 4, 4], "width {width}");
            let body = &texts[2];
            let left = field(body, &w, 0);
            let centre = field(body, &w, 1);
            let right = field(body, &w, 2);
            assert_eq!(left, "x   ", "width {width}");
            assert_eq!(
                centre, " x  ",
                "width {width}: the odd column goes to the right"
            );
            assert_eq!(right, "   x", "width {width}");
            // The assertion discriminates: an implementation ignoring
            // `Alignment` renders all three the same way.
            assert_ne!(left, centre, "width {width}");
            assert_ne!(centre, right, "width {width}");
            assert_ne!(left, right, "width {width}");
        }

        // A wrapped cell in each column is padded on the same side on EVERY one
        // of its lines, not on its first alone. Thirty-column cells against a
        // 68- and a 48-column budget wrap at both mandated widths.
        let long = "alpha bravo charlie delta echo";
        assert_eq!(columns(long), 30);
        let wrapped_source =
            format!("| left | cent | rght |\n|:---|:--:|---:|\n| {long} | {long} | {long} |\n");
        for width in [58, 78] {
            let texts = non_blank(&lines(&wrapped_source, width));
            let w = allocated_widths(&texts[1]);
            let body = &texts[2..];
            assert!(body.len() > 1, "width {width}: the cells must wrap");
            for text in body {
                for (j, cell_width) in w.iter().enumerate() {
                    let f = field(text, &w, j);
                    assert_eq!(columns(&f), *cell_width, "width {width}, column {j}");
                    if f.trim().is_empty() {
                        continue;
                    }
                    let lead = f.len() - f.trim_start().len();
                    let trail = f.len() - f.trim_end().len();
                    match j {
                        0 => assert_eq!(lead, 0, "width {width}: left column {f:?}"),
                        1 => assert!(
                            trail >= lead && trail - lead <= 1,
                            "width {width}: centred column {f:?}"
                        ),
                        _ => assert_eq!(trail, 0, "width {width}: right column {f:?}"),
                    }
                }
            }
        }
    }

    /// `markdown-render` :: "A ragged table renders every declared column and
    /// drops no header column".
    ///
    /// A regression guard on **pulldown-cmark's own normalisation**, not on logic
    /// this module adds: measured against 0.13.4, a body row with fewer cells
    /// than the delimiter row declares arrives already padded with empty cells
    /// and one with more arrives already truncated, so `fold` never observes a
    /// ragged row. The event-stream leg below is what says which component holds
    /// the property; it fails if a future parser version stops doing that.
    #[test]
    fn a_ragged_table_keeps_its_declared_columns() {
        let source = "| a | b | c |\n|---|---|---|\n| p | q |\n| r | s | t | u | v |\n";

        let mut cells_per_body_row: Vec<usize> = Vec::new();
        let mut count = 0usize;
        let mut in_row = false;
        for event in Parser::new_ext(
            source,
            Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS,
        ) {
            match event {
                Event::Start(Tag::TableRow) => {
                    in_row = true;
                    count = 0;
                }
                Event::Start(Tag::TableCell) if in_row => count += 1,
                Event::End(TagEnd::TableRow) => {
                    in_row = false;
                    cells_per_body_row.push(count);
                }
                _ => {}
            }
        }
        assert_eq!(
            cells_per_body_row,
            vec![3, 3],
            "pulldown-cmark no longer normalises a ragged row to the declared column count"
        );

        for width in [58, 78] {
            let texts = non_blank(&lines(source, width));
            let w = allocated_widths(&texts[1]);
            assert_eq!(w, vec![1, 1, 1], "width {width}");
            let total = 3 * w.len() + 1 + w.iter().sum::<usize>();
            for text in &texts {
                assert_eq!(columns(text), total, "width {width}: {text:?}");
                assert_eq!(
                    text.chars()
                        .filter(|c| ['│', '├', '┼', '┤'].contains(c))
                        .count(),
                    4,
                    "width {width}: {text:?} does not hold n + 1 separators"
                );
            }
            assert_eq!(
                field(&texts[2], &w, 2),
                " ".repeat(w[2]),
                "width {width}: the short row's third column"
            );
            let joined = texts.join("\n");
            for surplus in ['u', 'v'] {
                assert!(
                    !joined.contains(surplus),
                    "width {width}: the long row's surplus cell {surplus:?} was rendered"
                );
            }
        }
    }

    /// `markdown-render` :: "A region too narrow for the pipe grammar renders one
    /// cell per line". `4n + 1` is 9 for this two-column table, so the threshold
    /// falls inside the 0-through-10 sweep and is exercised rather than assumed.
    #[test]
    fn a_narrow_region_renders_one_cell_per_line() {
        let source = "| Gate | Runner |\n|---|---|\n| Format | cargo fmt |\n";

        assert!(lines(source, 0).is_empty());
        for width in 1..=8u16 {
            let texts = non_blank(&lines(source, width));
            for text in &texts {
                assert!(
                    !text.contains('│'),
                    "width {width}: the pipe grammar cannot fit here: {text:?}"
                );
                assert!(columns(text) <= width as usize, "width {width}: {text:?}");
            }
        }
        // At the widest width the fallback still covers, every cell is whole and
        // on its own line, in row-major order — the property the sweep above can
        // only bound.
        // Added during Change Review: the fallback's header cells "still
        // carrying `strong`" had no assertion at all, so emitting every segment
        // plain left the suite green.
        let at8: Vec<(String, bool)> = lines(source, 8)
            .iter()
            .map(|l| (l.text(), l.segments.iter().all(|s| s.face.strong)))
            .filter(|(text, _)| !text.is_empty())
            .collect();
        assert_eq!(
            at8,
            vec![
                ("Gate".to_string(), true),
                ("Runner".to_string(), true),
                ("Format".to_string(), false),
                ("cargo".to_string(), false),
                ("fmt".to_string(), false),
            ],
            "the fallback keeps the header row's `strong` and gives no body cell one"
        );
        for width in 9..=10u16 {
            let texts = non_blank(&lines(source, width));
            assert!(
                texts.iter().any(|t| t.contains('│')),
                "width {width}: the pipe grammar fits from 4n + 1 = 9 onward"
            );
            for text in &texts {
                assert!(columns(text) <= width as usize, "width {width}: {text:?}");
            }
        }
        for width in [58, 78] {
            let texts = non_blank(&lines(source, width));
            assert_eq!(
                texts,
                vec![
                    "│ Gate   │ Runner    │".to_string(),
                    "├────────┼───────────┤".to_string(),
                    "│ Format │ cargo fmt │".to_string(),
                ],
                "width {width}: the aligned form"
            );
        }
    }

    /// `markdown-render` :: "A struck run carries the face and composes with the
    /// others". The paragraph is short enough to fit on one line at 58 and at
    /// 78, so a wrap cannot split the phrases and the composition is what is
    /// under test.
    #[test]
    fn a_struck_run_carries_the_face_and_composes() {
        let source = "A ~~struck~~ word, ~~**struck bold**~~, and `~~x~~` in code.\n";
        for width in [58, 78] {
            let rendered = lines(source, width);
            let segments: Vec<&Segment> = rendered.iter().flat_map(|l| &l.segments).collect();

            let struck = segments
                .iter()
                .find(|s| s.text == "struck")
                .unwrap_or_else(|| panic!("width {width}: no `struck` segment"));
            assert_eq!(
                struck.face,
                Face {
                    strikethrough: true,
                    ..Face::plain()
                },
                "width {width}: a struck run carries that flag and no other"
            );

            let bold = segments
                .iter()
                .find(|s| s.text == "struck bold")
                .unwrap_or_else(|| panic!("width {width}: no `struck bold` segment"));
            assert!(bold.face.strikethrough && bold.face.strong, "width {width}");

            // A code span's content is verbatim: the `~` characters are present
            // in its text and the face is `code`, never `strikethrough`.
            let code = segments
                .iter()
                .find(|s| s.face.code)
                .unwrap_or_else(|| panic!("width {width}: no code segment"));
            assert_eq!(code.text, "~~x~~", "width {width}");
            assert!(!code.face.strikethrough, "width {width}");

            // The markers are consumed rather than rendered everywhere else.
            for seg in &segments {
                if seg.face.code {
                    continue;
                }
                assert!(
                    !seg.text.contains('~'),
                    "width {width}: {:?} still holds a marker",
                    seg.text
                );
            }
        }
    }

    /// `markdown-render` :: "A struck run split across a wrap keeps its face on
    /// both lines". The two expected strings are the ones
    /// `paragraph_wraps_at_58_and_78` names, which is what says the face costs
    /// no columns.
    #[test]
    fn a_struck_run_split_across_a_wrap_keeps_its_face() {
        let source = "~~alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo \
                      lima mike november oscar papa~~";
        let at58 = lines(source, 58);
        assert_eq!(
            text_of(&at58),
            vec![
                "alpha bravo charlie delta echo foxtrot golf hotel india".to_string(),
                "juliett kilo lima mike november oscar papa".to_string(),
            ]
        );
        let at78 = lines(source, 78);
        assert_eq!(
            text_of(&at78),
            vec![
                "alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima mike"
                    .to_string(),
                "november oscar papa".to_string(),
            ]
        );
        for (width, out) in [(58, &at58), (78, &at78)] {
            assert_eq!(out.len(), 2, "width {width}");
            for line in out.iter() {
                for seg in &line.segments {
                    assert!(
                        seg.face.strikethrough,
                        "width {width}: {:?} lost the face across the wrap",
                        seg.text
                    );
                }
            }
        }
    }

    /// `markdown-render` :: "A table renders as aligned columns sized to the
    /// region" -> "A table inside a container".
    ///
    /// **Added during Change Review.** The container-prefix branch of
    /// `emit_table` was implemented and normatively specified — "every emitted
    /// line SHALL carry that prefix, continuation lines included", and "`total`
    /// is then measured against the reduced width" — but no fixture in the tree
    /// put a table inside a quote or an item, so replacing the prefix push with
    /// a discard left every test green.
    #[test]
    fn a_table_inside_a_container_carries_the_prefix_on_every_line() {
        let fits = "| Gate | Runner |\n|---|---|\n| Format | cargo fmt |\n";
        let quoted: String = fits.lines().map(|l| format!("> {l}\n")).collect();
        let item: String = fits
            .lines()
            .enumerate()
            .map(|(i, l)| {
                if i == 0 {
                    format!("- {l}\n")
                } else {
                    format!("  {l}\n")
                }
            })
            .collect();

        for width in [58, 78] {
            // A block quote: every line, continuations included, carries `> `.
            let quoted_lines = non_blank(&lines(&quoted, width));
            assert_eq!(quoted_lines.len(), 3, "width {width}");
            for text in &quoted_lines {
                assert!(text.starts_with("│ "), "width {width}: {text:?}");
                assert_eq!(columns(text), 2 + 22, "width {width}: {text:?}");
                assert!(columns(text) <= width as usize, "width {width}");
            }

            // A list item: the item's own marker on the first line and its
            // hanging indent on the rest, exactly as a flow block's lines carry
            // them.
            let item_lines = non_blank(&lines(&item, width));
            assert_eq!(item_lines.len(), 3, "width {width}");
            assert!(item_lines[0].starts_with("• "), "width {width}");
            for text in &item_lines[1..] {
                assert!(
                    text.starts_with("  ") && !text.starts_with("• "),
                    "width {width}: {text:?}"
                );
            }
            for text in &item_lines {
                assert_eq!(columns(text), 2 + 22, "width {width}: {text:?}");
            }
        }

        // `total` is measured against the **reduced** width, not the region's:
        // the same table nested in a quote allocates two fewer columns than it
        // does at the top level, which is what makes the "at most `width`"
        // promise hold through a container. This is the leg that fails if the
        // prefix is carried but the width is not reduced for it.
        let cell_text = "alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo \
                         lima mike november oscar papa";
        let wide = format!("| key | value |\n|---|---|\n| k | {cell_text} |\n");
        let wide_quoted: String = wide.lines().map(|l| format!("> {l}\n")).collect();
        for width in [58, 78] {
            let plain = non_blank(&lines(&wide, width));
            let nested = non_blank(&lines(&wide_quoted, width));
            assert_eq!(
                allocated_widths(&plain[1]),
                vec![3, width as usize - 10],
                "width {width}: the top-level allocation"
            );
            assert_eq!(
                allocated_widths(nested[1].strip_prefix("│ ").expect("the quote prefix")),
                vec![3, width as usize - 12],
                "width {width}: the nested allocation must lose the prefix's columns"
            );
            for text in &nested {
                assert!(text.starts_with("│ "), "width {width}: {text:?}");
                assert_eq!(columns(text), width as usize, "width {width}: {text:?}");
            }
        }
    }

    #[test]
    fn line_text_concatenates_its_segments() {
        let source = "Plain **bold** and *italic* text.\n";
        let at58 = text_of(&lines(source, 58));
        let at78 = text_of(&lines(source, 78));
        let non_blank58: Vec<&String> = at58.iter().filter(|s| !s.is_empty()).collect();
        let non_blank78: Vec<&String> = at78.iter().filter(|s| !s.is_empty()).collect();
        assert_eq!(non_blank58.len(), 1);
        assert_eq!(non_blank78.len(), 1);
        assert_eq!(non_blank58[0], non_blank78[0]);
        assert_eq!(non_blank58[0], "Plain bold and italic text.");

        // Pin the concatenation directly against the segments, at both
        // widths, so this is the test every other assertion in this
        // module reads Line::text() through.
        for width in [58, 78] {
            let out = lines(source, width);
            let line = out.iter().find(|l| !l.segments.is_empty()).unwrap();
            let manual: String = line.segments.iter().map(|s| s.text.as_str()).collect();
            assert_eq!(line.text(), manual, "width {width}");
        }
    }

    #[test]
    fn a_loose_list_keeps_its_marker_and_no_blank_between_items() {
        // Change Review finding: a single blank line anywhere in a bullet
        // list makes CommonMark treat the WHOLE list as loose, so every
        // item — even a one-line one — arrives as Start(Item) ->
        // Start(Paragraph) rather than Start(Item) -> Text directly. A
        // fold that reset to the ambient (unprefixed) configuration on
        // every Start(Paragraph) discarded the item's own marker the
        // moment this happened; markdown-render's "Items SHALL NOT be
        // separated by a blank line, whether the source list is tight or
        // loose" and "A bullet item SHALL be marked `- `" both hold
        // regardless.
        let source = "- one\n\n- two\n\n- three\n";
        for width in [58, 78] {
            let texts: Vec<String> = text_of(&lines(source, width))
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect();
            assert_eq!(texts, vec!["• one", "• two", "• three"], "width {width}");
            let out = lines(source, width);
            assert!(
                !out.iter().any(|l| l.segments.is_empty()),
                "width {width}: a loose list must not gain a blank line between its items"
            );
        }
    }

    // `task-item-bodies` :: `ui::markdown::inline` — a second entry point
    // that renders a fragment as one paragraph and recognises no block
    // construct (design.md -> Decision 4).

    #[test]
    fn a_fragments_inline_faces_are_set_and_its_text_is_unchanged() {
        let text = "2.2 GREEN: add the `Recorder` implementation, **bolded**, and *stressed*";
        for width in [78, 58] {
            let out = inline(text, width);
            let segments: Vec<&Segment> = out.iter().flat_map(|l| l.segments.iter()).collect();
            let concatenated: String = segments.iter().map(|s| s.text.as_str()).collect();
            assert!(
                concatenated.contains("Recorder"),
                "width {width}: {concatenated:?}"
            );
            assert!(
                concatenated.contains("bolded"),
                "width {width}: {concatenated:?}"
            );
            assert!(
                concatenated.contains("stressed"),
                "width {width}: {concatenated:?}"
            );

            let code = segments
                .iter()
                .find(|s| s.text == "Recorder")
                .unwrap_or_else(|| panic!("width {width}: no Recorder segment"));
            assert!(code.face.code, "width {width}");

            let bolded = segments
                .iter()
                .find(|s| s.text == "bolded")
                .unwrap_or_else(|| panic!("width {width}: no bolded segment"));
            assert!(bolded.face.strong, "width {width}");

            let stressed = segments
                .iter()
                .find(|s| s.text == "stressed")
                .unwrap_or_else(|| panic!("width {width}: no stressed segment"));
            assert!(stressed.face.emphasis, "width {width}");

            for seg in &segments {
                assert_eq!(seg.face.heading, None, "width {width}: {seg:?}");
            }
        }
    }

    #[test]
    fn a_leading_block_marker_is_literal_text_not_a_block() {
        let fragments = [
            "# not a heading",
            "- not a bullet",
            "> not a quote",
            "1. not an ordered list",
            "--- not a rule",
        ];
        for width in [78, 58] {
            let mut differences = 0;
            for fragment in fragments {
                let out = inline(fragment, width);
                let full: String = out.iter().map(Line::text).collect();
                assert_eq!(full, fragment, "width {width}: {fragment:?}");
                for line in &out {
                    for seg in &line.segments {
                        assert_eq!(
                            seg.face.heading, None,
                            "width {width}: {fragment:?} {seg:?}"
                        );
                    }
                }
                if lines(fragment, width) != out {
                    differences += 1;
                }
            }
            assert!(
                differences >= 4,
                "width {width}: only {differences} of {} fragments differed from lines()",
                fragments.len()
            );
        }
    }

    #[test]
    fn a_fragment_wraps_and_hard_splits_exactly_as_a_paragraph_does() {
        let prose = "alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima \
                      mike november oscar papa quebec romeo sierra tango uniform victor whiskey \
                      xray yankee zulu alpha bravo charlie delta echo foxtrot golf hotel india \
                      juliett kilo lima mike november oscar papa quebec romeo sierra tango";
        for width in [78, 58] {
            let out = inline(prose, width);
            assert_eq!(out, lines(prose, width), "width {width}");
            for line in &out {
                assert!(
                    columns(&line.text()) <= width as usize,
                    "width {width}: {:?}",
                    line.text()
                );
            }
        }
        let at58 = inline(prose, 58).len();
        let at78 = inline(prose, 78).len();
        assert!(at58 > at78, "58: {at58} lines, 78: {at78} lines");

        let long_word = "x".repeat(150);
        for width in [78, 58] {
            let out = inline(&long_word, width);
            assert_eq!(
                out,
                lines(&long_word, width),
                "width {width}: hard split parity"
            );
            assert!(
                out.len() > 1,
                "width {width}: {} chars should hard-split",
                long_word.len()
            );
            for line in &out {
                assert!(
                    columns(&line.text()) <= width as usize,
                    "width {width}: {:?}",
                    line.text()
                );
            }
        }
    }

    /// `markdown-render` :: "`inline` SHALL recognise **no block
    /// construct**" and "every returned `Face` SHALL carry `heading:
    /// None`", over the inputs a first-line-only, whitespace-blind escape
    /// was measured to fail on. The production caller cannot reach these
    /// today — `tasks::parse` trims an item's text and splits it on `\n` —
    /// but `inline` is `pub` and its requirement is unconditional, so the
    /// guarantee is asserted over the inputs rather than over the caller.
    #[test]
    fn a_whitespace_led_or_multi_line_fragment_still_opens_no_block() {
        for width in [78, 58] {
            for input in [
                "   --- ",
                "  ``` fenced",
                "\t### heading",
                "foo\n===",
                "foo\n---",
                "para\n\n# heading",
                "a\n\n- bullet",
            ] {
                let out = inline(input, width);
                assert!(
                    !out.is_empty(),
                    "width {width}: {input:?} rendered nothing at all"
                );
                for line in &out {
                    assert!(
                        !line.text().contains('\\'),
                        "width {width}: {input:?} leaked a backslash: {:?}",
                        line.text()
                    );
                    for seg in &line.segments {
                        assert_eq!(
                            seg.face.heading, None,
                            "width {width}: {input:?} set a heading face: {seg:?}"
                        );
                    }
                }
                let joined: String = out.iter().map(Line::text).collect::<Vec<_>>().join("\n");
                for marker in ["---", "```", "###", "===", "#", "-"] {
                    if input.contains(marker) {
                        assert!(
                            joined.contains(marker),
                            "width {width}: {input:?} lost {marker:?}, got {joined:?}"
                        );
                    }
                }
                assert!(
                    !joined.contains('\u{2022}'),
                    "width {width}: {input:?} gained a bullet glyph: {joined:?}"
                );
            }
        }
    }

    #[test]
    fn empty_whitespace_and_zero_width_inputs_return_nothing() {
        assert_eq!(inline("", 58), Vec::new());
        assert_eq!(inline("", 78), Vec::new());
        assert_eq!(inline("   ", 58), Vec::new());
        assert_eq!(inline("   ", 78), Vec::new());
        assert_eq!(inline("hello world", 0), Vec::new());
    }
}
