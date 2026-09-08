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

/// One block-level unit to lay out. `groups` holds one `Vec<Run>` per
/// rendered "hard line" — a paragraph's soft-break-delimited segment, or
/// one verbatim source line of a code or HTML block — and every group
/// always starts a fresh output row, which is what the soft-break and
/// verbatim-line rules require.
#[derive(Debug, Clone)]
struct Block {
    /// A thematic break: laid out as a single line of `width` dashes,
    /// ignoring every other field below.
    is_rule: bool,
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
            is_rule: false,
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
            is_rule: true,
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
        self.finish();
        self.blocks
    }
}

/// Fold `source`'s event stream into a `Vec<Block>` — plain data carrying
/// its own indent, marker, and inline runs, but nothing width-dependent.
/// Every block-level construct `Options::empty()` can produce is handled:
/// paragraphs, headings, lists (nested, ordered from their own start
/// value), code blocks (fenced or indented, verbatim), block quotes
/// (nested), thematic breaks, and raw HTML (block and inline), each
/// verbatim. A table, footnote, strikethrough, or task-list source is not
/// modelled by `Options::empty()` at all — it arrives as ordinary
/// paragraph text, which is the literal-text degraded state
/// `markdown-render` states.
fn fold(source: &str) -> Vec<Block> {
    let mut f = Folder::new();
    for event in Parser::new_ext(source, Options::empty()) {
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
                // `FootnoteDefinition`, the definition-list tags, the
                // table tags, `Superscript`, `Subscript`, `Strikethrough`,
                // and `MetadataBlock` cannot be produced by
                // `Options::empty()`. The wildcard is the stated default:
                // total over the enum, and a future pulldown-cmark variant
                // reaches it rather than a missing-arm compile error
                // changing this module's shape.
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Paragraph | TagEnd::Heading(_) => f.finish(),
                TagEnd::List(_) => f.end_list(),
                TagEnd::Item | TagEnd::CodeBlock | TagEnd::HtmlBlock => f.finish(),
                TagEnd::BlockQuote(_) => f.end_quote(),
                TagEnd::Emphasis | TagEnd::Strong | TagEnd::Link => f.pop_faced(),
                TagEnd::Image => f.end_image(),
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
            // `DisplayMath` cannot be produced by `Options::empty()`. The
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
    if block.is_rule {
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
        assert_eq!(first78.chars().count(), 78);
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
        assert_eq!(first58.chars().count(), 57);
        let at78 = lines(source, 78);
        let first78 = at78[0].text();
        assert_eq!(
            first78,
            "- alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima"
        );
        assert_eq!(first78.chars().count(), 75);
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
            at58.iter().map(|s| s.chars().count()).collect::<Vec<_>>(),
            vec![58, 58, 14]
        );
        assert_eq!(at58.concat(), "x".repeat(130));

        let at78: Vec<String> = text_of(&lines(&source, 78))
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(
            at78.iter().map(|s| s.chars().count()).collect::<Vec<_>>(),
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
        assert_eq!(first58.chars().count(), 57);

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
            assert_eq!(rule.text().chars().count(), width as usize);
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
            (
                "GFM table row",
                "| Gate | Runner |\n|---|---|\n| Format | cargo fmt |\n",
            ),
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

    #[test]
    fn a_table_renders_as_literal_source_rows() {
        let source = "| Gate | Runner |\n|---|---|\n| Format | cargo fmt |\n";
        for width in [58, 78] {
            let texts: Vec<String> = text_of(&lines(source, width))
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect();
            assert_eq!(
                texts,
                vec!["| Gate | Runner |", "|---|---|", "| Format | cargo fmt |"],
                "width {width}"
            );
            for line in lines(source, width)
                .iter()
                .filter(|l| !l.segments.is_empty())
            {
                for seg in &line.segments {
                    assert_eq!(seg.face, Face::plain());
                }
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
            assert_eq!(texts, vec!["- one", "- two", "- three"], "width {width}");
            let out = lines(source, width);
            assert!(
                !out.iter().any(|l| l.segments.is_empty()),
                "width {width}: a loose list must not gain a blank line between its items"
            );
        }
    }
}
