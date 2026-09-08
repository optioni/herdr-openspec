//! Markdown source to plain-data lines of faced segments, parameterised by
//! an interior width. A pure total transformation: no filesystem, process,
//! environment, network, or standard-I/O API, and it panics for no `&str`
//! and no `u16`. Styling a segment for the view is not this module's job —
//! see `openspec/changes/markdown-viewer/specs/markdown-render/spec.md`.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use crate::ui::layout::{columns, truncate_columns};

/// A segment's styling. A struct of flags, not an enum: markdown nests
/// (`[**bold link**](x)` is bold *and* a link), and an enum would force an
/// arbitrary precedence rule. `heading` is `Option<u8>` because the level
/// is real information the marker carries, not a flag. The only type in
/// this module that derives `Default`: a `Face` is a value with a
/// meaningful zero (unstyled), not a state type `NODEFAULT-UI` gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Face {
    pub heading: Option<u8>,
    pub strong: bool,
    pub emphasis: bool,
    pub code: bool,
    pub link: bool,
    pub quoted: bool,
}

impl Face {
    /// The all-`false`, `heading: None` value.
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
/// rendered "hard line" — a paragraph's soft-break-delimited segment, or
/// one verbatim source line of a code or HTML block — and every group
/// always starts a fresh output row, which is what the soft-break and
/// verbatim-line rules require.
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

/// `"> "` repeated once per block-quote nesting level. Empty outside a
/// quote.
fn quote_prefix(depth: usize) -> String {
    "> ".repeat(depth)
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

    // `Some` between `Start(Table)` and `End(Table)`: the rows accumulated
    // so far, the current row's cells, and whether that row is the header.
    // The current *cell*'s runs accumulate in `group`, exactly as a
    // paragraph's do, which is what lets a cell carry inline faces without
    // a second accumulation path.
    table: Option<TableBuilder>,
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
            table: None,
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

    /// Close the current line-group (a soft/hard break, or the block's own
    /// end) and start a new one.
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

    /// `Tag::Item`: the marker (`- ` or the list's own sequential number),
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
            "- ".to_string()
        };
        let indent = "  ".repeat(frame.depth);
        let qp = quote_prefix(self.quote_depth);
        self.first_prefix = format!("{qp}{indent}{marker}");
        self.cont_prefix = format!("{qp}{}", " ".repeat(columns(&indent) + columns(&marker)));
        self.prefix_face = Face::plain();
        self.category = Category::Item;
        self.hard_split = false;
        let base = self.quoted_base();
        self.faces = vec![base];
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
/// `ENABLE_TABLES` on — GFM pipe tables. A footnote or task-list source is
/// not modelled by this option set at all: it arrives as ordinary paragraph
/// text, which is the literal-text degraded state `markdown-render` states.
///
/// The option set is exactly `ENABLE_TABLES | ENABLE_STRIKETHROUGH` and
/// nothing else (design.md -> Decision 1): a further flag turned on here
/// starts emitting events into the wildcards below, where a construct
/// **vanishes** rather than degrading to its literal text.
fn fold(source: &str) -> Vec<Block> {
    let mut f = Folder::new();
    for event in Parser::new_ext(source, Options::ENABLE_TABLES) {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => f.start_paragraph(),
                Tag::Heading { level, .. } => f.start_heading(level as u8),
                Tag::List(start) => f.start_list(start),
                Tag::Item => f.start_item(),
                Tag::BlockQuote(_) => f.start_quote(),
                Tag::CodeBlock(_) | Tag::HtmlBlock => f.start_verbatim_block(),
                Tag::Emphasis => f.push_faced(|face| face.emphasis = true),
                Tag::Strong => f.push_faced(|face| face.strong = true),
                Tag::Link { .. } => f.push_faced(|face| face.link = true),
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
                TagEnd::Emphasis | TagEnd::Strong | TagEnd::Link => f.pop_faced(),
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
            Event::SoftBreak | Event::HardBreak => f.group_break(),
            Event::Rule => f.push_rule(),
            // `FootnoteReference`, `TaskListMarker`, `InlineMath`, and
            // `DisplayMath` cannot be produced by this option set —
            // `ENABLE_FOOTNOTES` and `ENABLE_TASKLISTS` stay off precisely
            // so the first two keep arriving as literal text instead. The
            // wildcard is the stated default: total over the enum, and a
            // future pulldown-cmark variant reaches it rather than a
            // missing-arm compile error changing this module's shape.
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
                text: "-".repeat(width as usize),
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
            append(&mut segs, "|", Face::plain());
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
                append(&mut segs, "|", Face::plain());
            }
            out.push(segs);
        }
        if row.header {
            let mut segs: Vec<Segment> = Vec::new();
            append(&mut segs, "|", Face::plain());
            for cell_width in w {
                append(&mut segs, &"-".repeat(cell_width + 2), Face::plain());
                append(&mut segs, "|", Face::plain());
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
            if cell.iter().all(|r| r.text.trim().is_empty()) {
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

/// One atom of a word-wrappable stream: a contiguous run of non-space
/// characters carrying one face, or a space (a break opportunity, never
/// itself rendered — a single space is synthesised between two words that
/// need one).
enum Atom {
    Word(String, Face),
    /// A space, carrying the face of the run it was found in — the run
    /// that contains a word and its own adjacent space is what a
    /// multi-word faced phrase (`the design`, all one `Link` run) is
    /// built from, so the space merges into the same segment as its
    /// neighbours rather than forcing a break at every face boundary.
    Space(Face),
}

fn atomize(group: &[Run]) -> Vec<Atom> {
    let mut atoms = Vec::new();
    for run in group {
        let mut current = String::new();
        for ch in run.text.chars() {
            if ch == ' ' {
                if !current.is_empty() {
                    atoms.push(Atom::Word(std::mem::take(&mut current), run.face));
                }
                atoms.push(Atom::Space(run.face));
            } else {
                current.push(ch);
            }
        }
        if !current.is_empty() {
            atoms.push(Atom::Word(current, run.face));
        }
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
            Atom::Word(text, face) => {
                let mut remaining = text.as_str();
                loop {
                    let word_cols = columns(remaining);
                    if current_len == 0 && word_cols > width {
                        let (chunk, rest) = split_at_columns(remaining, width);
                        if !chunk.is_empty() {
                            append(&mut current, chunk, face);
                        }
                        lines.push(std::mem::take(&mut current));
                        current_len = 0;
                        pending_space = None;
                        remaining = rest;
                        if remaining.is_empty() {
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
                        append(&mut current, remaining, face);
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

    fn composite_fixture() -> String {
        // Heading, paragraph, bullet list, nested list, fenced code, block
        // quote, thematic break, link — this test only needs to prove
        // totality and width compliance, not per-construct styling (the
        // dedicated tests below own that). Each block carries one
        // distinctive one-word marker so a check can find it regardless of
        // where a wrap point lands.
        "# headingword marker\n\
         \n\
         paragraphword marker text padded out with extra words so the line wraps at both widths under test here today\n\
         \n\
         - bulletword marker text padded with extra words so the item wraps under both widths under test\n  - nestedword marker text padded with extra words so the item wraps under both widths under test\n\
         \n\
         ```sh\n\
         codeword marker text padded with extra words so the block wraps under both widths under test today\n\
         ```\n\
         \n\
         > quoteword marker text padded with extra words so the quote wraps under both widths under test today\n\
         \n\
         ---\n\
         \n\
         See [linkword marker](design.md) for more padded words so this paragraph also wraps under test today.\n"
            .to_string()
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
            assert!(
                !text.contains("design.md"),
                "link destination leaked at {width}"
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
        let source = "first line\nsecond line\n";
        for width in [58, 78] {
            let out = text_of(&lines(source, width));
            let non_blank: Vec<&String> = out.iter().filter(|s| !s.is_empty()).collect();
            assert_eq!(non_blank, vec!["first line", "second line"]);
            assert!(!out.iter().any(|s| s == "first line second line"));
        }
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
            assert!(non_blank[0].text().starts_with("- alpha"), "width {width}");
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
            "- alpha bravo charlie delta echo foxtrot golf hotel india"
        );
        assert_eq!(columns(&first58), 57);
        let at78 = lines(source, 78);
        let first78 = at78[0].text();
        assert_eq!(
            first78,
            "- alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima"
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
                vec!["- outer", "  - inner", "    - deepest"],
                "width {width}"
            );
        }

        let wrapped = "- outer\n  - inner alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima mike november\n";
        for width in [58, 78] {
            let out = lines(wrapped, width);
            let non_blank: Vec<&Line> = out.iter().filter(|l| !l.segments.is_empty()).collect();
            let inner_start = non_blank
                .iter()
                .position(|l| l.text().starts_with("  - inner"))
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
                assert!(text.starts_with("> "), "width {width}: {text:?}");
                for seg in &line.segments {
                    assert!(seg.face.quoted, "width {width}: {seg:?}");
                }
            }
        }
        let at58 = lines(source, 58);
        let first58 = at58.iter().find(|l| !l.segments.is_empty()).unwrap().text();
        assert_eq!(
            first58,
            "> alpha bravo charlie delta echo foxtrot golf hotel india"
        );
        assert_eq!(columns(&first58), 57);

        let nested = "> > nested alpha bravo\n";
        for width in [58, 78] {
            let out = lines(nested, width);
            let first = out.iter().find(|l| !l.segments.is_empty()).unwrap().text();
            assert!(first.starts_with("> > "), "width {width}: {first:?}");
        }
    }

    #[test]
    fn a_thematic_break_fills_the_width() {
        let source = "alpha before\n\n---\n\nbravo after\n";
        for width in [58, 78] {
            let out = lines(source, width);
            let idx = out
                .iter()
                .position(|l| l.segments.len() == 1 && l.segments[0].text.chars().all(|c| c == '-'))
                .unwrap_or_else(|| panic!("width {width}: no rule line found"));
            let rule = &out[idx];
            assert_eq!(columns(&rule.text()), width as usize);
            assert_eq!(rule.segments[0].face, Face::plain());
            assert!(out[idx - 1].segments.is_empty(), "width {width}");
            assert!(out[idx + 1].segments.is_empty(), "width {width}");
        }
    }

    /// `degraded-coverage` :: "A footnote, strikethrough, and a table each render as literal
    /// source" — row 12. Four constructs the parser does not model — a footnote reference
    /// and its definition, strikethrough, a GFM table row, and a task-list item — each on a
    /// tab this test never marks tracked, so `ui::markdown::lines` (not `ui::tasks::lines`)
    /// is what renders them: rendered line count equals source line count, one line per
    /// source line, at both mandated markdown widths.
    #[test]
    fn unmodelled_constructs_render_as_source() {
        let sources = [
            (
                "footnote reference and definition",
                "See it here[^1].\n\n[^1]: The note.\n",
            ),
            ("strikethrough", "~~gone~~ text.\n"),
            ("task-list item", "- [ ] an item\n- [x] a done item\n"),
        ];
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

        // The tracked-tasks carve-out this row's own wording names ("on a tab **other**
        // than the tracked-tasks one"): the SAME task-list source, dispatched through
        // `ui::tasks::lines` (what `ui::detail::content_lines` reaches for a tracked tab)
        // instead of `ui::markdown::lines`, renders the checklist grammar — a progress bar
        // and `[ ]`/`[x]` glyphs — rather than the literal source text, discriminating this
        // test's own claim that the OTHER tabs render literally.
        let task_list_source = "- [ ] an item\n- [x] a done item\n";
        let progress = crate::tasks::Progress {
            completed: 1,
            total: 2,
        };
        for width in [58u16, 78u16] {
            let tracked = crate::ui::tasks::lines(task_list_source, &progress, width);
            let tracked_text: Vec<String> = tracked
                .iter()
                .map(|l| l.text().trim_end().to_string())
                .collect();
            assert!(
                tracked_text
                    .iter()
                    .any(|l| l.contains('[') && l.contains(']')),
                "width {width}: the tracked-tasks tab must render the checklist grammar, \
                 not literal source: {tracked_text:?}"
            );
            assert_ne!(
                tracked_text,
                text_of(&lines(task_list_source, width)),
                "width {width}: the tracked tab must differ from the markdown rendering, or \
                 the carve-out proves nothing"
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
    /// `-` between two `|` measures `w[j] + 2`. Read back out of the output
    /// rather than recomputed by the test, so an assertion on it is an assertion
    /// on what the reader is shown.
    fn allocated_widths(delimiter: &str) -> Vec<usize> {
        delimiter
            .trim_matches('|')
            .split('|')
            .map(|run| columns(run) - 2)
            .collect()
    }

    /// Column `j`'s own field of a rendered row line: the `w[j]` columns between
    /// that column's two padding spaces. The leading `|` costs one column and
    /// every earlier column costs `w[i] + 3` — a padding space, its content, a
    /// padding space, and the `|` that closes it.
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
        for width in [58u16, 78u16] {
            let rendered = lines(source, width);
            let texts = non_blank(&rendered);
            assert_eq!(
                texts,
                vec![
                    "| Gate   | Runner       |".to_string(),
                    "|--------|--------------|".to_string(),
                    "| Format | cargo fmt    |".to_string(),
                    "| Lint   | cargo clippy |".to_string(),
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

            // The columns are aligned: the delimiter line's `|` offsets are every
            // row line's.
            let expected: Vec<usize> = texts[1]
                .chars()
                .enumerate()
                .filter(|(_, c)| *c == '|')
                .map(|(i, _)| i)
                .collect();
            assert_eq!(expected.len(), 3, "width {width}: n + 1 pipes");
            for text in &texts {
                let got: Vec<usize> = text
                    .chars()
                    .enumerate()
                    .filter(|(_, c)| *c == '|')
                    .map(|(i, _)| i)
                    .collect();
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
        for (width, expect_w) in [(58u16, vec![3usize, 48]), (78u16, vec![3, 68])] {
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
        for (width, expect_w) in [(58u16, vec![4usize, 4, 4, 33]), (78u16, vec![4, 4, 4, 53])] {
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
        for width in [58u16, 78u16] {
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
        for width in [58u16, 78u16] {
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
        for event in Parser::new_ext(source, Options::ENABLE_TABLES) {
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

        for width in [58u16, 78u16] {
            let texts = non_blank(&lines(source, width));
            let w = allocated_widths(&texts[1]);
            assert_eq!(w, vec![1, 1, 1], "width {width}");
            let total = 3 * w.len() + 1 + w.iter().sum::<usize>();
            for text in &texts {
                assert_eq!(columns(text), total, "width {width}: {text:?}");
                assert_eq!(
                    text.chars().filter(|c| *c == '|').count(),
                    4,
                    "width {width}: {text:?} does not hold n + 1 pipes"
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
                    !text.contains('|'),
                    "width {width}: the pipe grammar cannot fit here: {text:?}"
                );
                assert!(columns(text) <= width as usize, "width {width}: {text:?}");
            }
        }
        // At the widest width the fallback still covers, every cell is whole and
        // on its own line, in row-major order — the property the sweep above can
        // only bound.
        assert_eq!(
            non_blank(&lines(source, 8)),
            vec![
                "Gate".to_string(),
                "Runner".to_string(),
                "Format".to_string(),
                "cargo".to_string(),
                "fmt".to_string(),
            ]
        );
        for width in 9..=10u16 {
            let texts = non_blank(&lines(source, width));
            assert!(
                texts.iter().any(|t| t.contains('|')),
                "width {width}: the pipe grammar fits from 4n + 1 = 9 onward"
            );
            for text in &texts {
                assert!(columns(text) <= width as usize, "width {width}: {text:?}");
            }
        }
        for width in [58u16, 78u16] {
            let texts = non_blank(&lines(source, width));
            assert_eq!(
                texts,
                vec![
                    "| Gate   | Runner    |".to_string(),
                    "|--------|-----------|".to_string(),
                    "| Format | cargo fmt |".to_string(),
                ],
                "width {width}: the aligned form"
            );
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
            assert_eq!(texts, vec!["- one", "- two", "- three"], "width {width}");
            let out = lines(source, width);
            assert!(
                !out.iter().any(|l| l.segments.is_empty()),
                "width {width}: a loose list must not gain a blank line between its items"
            );
        }
    }
}
