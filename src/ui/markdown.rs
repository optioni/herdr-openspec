//! Markdown source to plain-data lines of faced segments, parameterised by
//! an interior width. A pure total transformation: no filesystem, process,
//! environment, network, or standard-I/O API, and it panics for no `&str`
//! and no `u16`. Styling a segment for the view is not this module's job —
//! see `openspec/changes/markdown-viewer/specs/markdown-render/spec.md`.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

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
    hard_split: bool,
    first_prefix: String,
    cont_prefix: String,
    prefix_face: Face,
    category: Category,
    groups: Vec<Vec<Run>>,
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
            hard_split: self.hard_split,
            first_prefix: std::mem::take(&mut self.first_prefix),
            cont_prefix: std::mem::take(&mut self.cont_prefix),
            prefix_face: self.prefix_face,
            category: self.category,
            groups,
        });
        self.reset_ambient();
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

    fn start_paragraph(&mut self) {
        self.finish();
        // Already the ambient configuration — nothing to set.
    }

    fn start_heading(&mut self, level: u8) {
        self.finish();
        let face = Face {
            heading: Some(level),
            ..Face::plain()
        };
        self.first_prefix = "#".repeat(level as usize) + " ";
        self.cont_prefix = " ".repeat(level as usize + 1);
        self.prefix_face = face;
        self.category = Category::Other;
        self.hard_split = false;
        self.faces = vec![face];
    }

    fn push_text(&mut self, text: &str) {
        if let Some(alt) = self.image_alt.as_mut() {
            alt.push_str(text);
            return;
        }
        let face = self.current_face();
        self.group.push(Run {
            text: text.to_string(),
            face,
        });
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
/// Group 4 lays out `Paragraph` and `Heading` exactly, per
/// `markdown-render`'s heading and inline-face requirements; every other
/// block-level construct (lists, code, quotes, rules, raw HTML) is
/// transparent here — its inline content still reaches the ambient
/// accumulation and is never dropped, but without a marker or a dedicated
/// face. Group 5 replaces this fallback with the real handling for each.
fn fold(source: &str) -> Vec<Block> {
    let mut f = Folder::new();
    for event in Parser::new_ext(source, Options::empty()) {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => f.start_paragraph(),
                Tag::Heading { level, .. } => f.start_heading(level as u8),
                Tag::Emphasis => f.push_faced(|face| face.emphasis = true),
                Tag::Strong => f.push_faced(|face| face.strong = true),
                Tag::Link { .. } => f.push_faced(|face| face.link = true),
                Tag::Image { .. } => f.start_image(),
                // List, Item, BlockQuote, CodeBlock, HtmlBlock: transparent
                // in this group. Their inner Text/Html events still reach
                // `push_text`/`push_code_span` through the ambient block.
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Paragraph | TagEnd::Heading(_) => f.finish(),
                TagEnd::Emphasis | TagEnd::Strong | TagEnd::Link => f.pop_faced(),
                TagEnd::Image => f.end_image(),
                _ => {}
            },
            Event::Text(text) => f.push_text(&text),
            Event::Code(text) => f.push_code_span(&text),
            Event::InlineHtml(text) | Event::Html(text) => f.push_text(&text),
            Event::SoftBreak | Event::HardBreak => f.group_break(),
            // `Rule` carries no text of its own and is not yet laid out —
            // group 5 gives it a dedicated fill. Dropping the bare event
            // here loses nothing that was ever rendered as text.
            Event::Rule => {}
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
    let prefix_len = block.first_prefix.chars().count() as u16;
    let content_width = width.saturating_sub(prefix_len);
    if content_width == 0 {
        let truncated = truncate_chars(&block.first_prefix, width as usize);
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

/// The first `n` characters of `s`, safe at any UTF-8 boundary.
fn truncate_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// Split `s` after its first `n` characters. `n` must be strictly less
/// than `s`'s character count, which every call site guarantees.
fn split_at_char(s: &str, n: usize) -> (&str, &str) {
    match s.char_indices().nth(n) {
        Some((idx, _)) => (&s[..idx], &s[idx..]),
        None => (s, ""),
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
                    let word_len = remaining.chars().count();
                    if current_len == 0 && word_len > width {
                        let (chunk, rest) = split_at_char(remaining, width);
                        append(&mut current, chunk, face);
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
                    if current_len + sep + word_len <= width {
                        if let Some(sep_face) = pending_space.take() {
                            append(&mut current, " ", sep_face);
                            current_len += 1;
                        }
                        append(&mut current, remaining, face);
                        current_len += word_len;
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
    let chars: Vec<char> = run.text.chars().collect();
    chars
        .chunks(width)
        .map(|c| {
            vec![Segment {
                text: c.iter().collect(),
                face: run.face,
            }]
        })
        .collect()
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
        // quote, thematic break, link — group 4's simplified fold treats
        // lists/code/quotes/rules as transparent, so this fixture only
        // needs to prove totality and width compliance, not their final
        // styling (group 5's tests own that). Each block carries one
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
        for width in [0u16, 1, 2, 3, 10, 58, 78, 200] {
            let out = lines(&source, width);
            for line in &out {
                assert!(
                    line.text().chars().count() <= width as usize,
                    "width {width}: line {:?} exceeds it",
                    line.text()
                );
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
        let pathological = [
            "#".to_string(),
            "```sh\nunclosed fence".to_string(),
            "[link(".to_string(),
            "x".repeat(500),
            "\n\n\n\n\n".to_string(),
            "     ".to_string(),
            "**bold".to_string(),
            "hold \u{0}my\u{301}beer".to_string(),
        ];
        for width in [58, 78] {
            for source in &pathological {
                let out = lines(source, width);
                for line in &out {
                    assert!(
                        line.text().chars().count() <= width as usize,
                        "width {width} source {source:?}: line {:?} exceeds it",
                        line.text()
                    );
                }
            }
            let hard = lines(&"x".repeat(500), width);
            assert!(
                hard.iter()
                    .all(|l| l.text().chars().count() <= width as usize)
            );
            assert!(
                hard.iter()
                    .any(|l| l.text().chars().count() == width as usize),
                "the 500-character token must be hard-split into full-width lines at {width}"
            );
        }
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
}
