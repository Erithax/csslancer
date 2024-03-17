use crate::interop::CssLancerRange;
use crate::parser::css_nodes::CssNodeTree;
use crate::parser::css_parser::Parser;
use lsp_types::Url;

use std::ops::Range;

pub struct Source {
    pub url: Url,
    pub version: i32,
    //pub text: String,
    // text: Prehashed<String>,
    // root: Prehashed<SyntaxNode>,
    pub tree: CssNodeTree,
    lines: Vec<Line>,
}

impl std::fmt::Debug for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Source {{
            url: {},
            version: {},
            text: ...,
            tree: ...,
            lines: ...,
        }}",
            self.url, self.version
        )
    }
}

impl Source {
    // Create a new source file.
    pub fn new(url: Url, text: String, version: i32) -> Self {
        Self {
            url,
            version,
            lines: Line::lines(&text),
            tree: Parser::new_with_text(text).into_stylesheet(),
        }
    }

    /// Create a source file without a real id and path, usually for testing.
    pub fn detached(text: impl Into<String>) -> Self {
        return Self::new(
            Url::parse("https://localhost/detached").unwrap(),
            text.into(),
            0,
        );
    }

    pub fn text(&self) -> &str {
        return &self.tree.1;
    }

    /// Slice out the part of the source code enclosed by the range.
    pub fn get(&self, range: Range<usize>) -> Option<&str> {
        self.text().get(range)
    }

    pub fn text_at(&self, csslancer_range: CssLancerRange) -> &str {
        &self.text()[csslancer_range]
    }

    /// Return the index of the UTF-16 code unit at the byte index.
    pub fn byte_to_utf16(&self, byte_idx: usize) -> Option<usize> {
        let line_idx = self.byte_to_line(byte_idx)?;
        let line = self.lines.get(line_idx)?;
        let head = self.text().get(line.utf8_offset..byte_idx)?;
        Some(line.utf16_offset + len_utf16(head))
    }

    /// Return the index of the line that contains the given byte index.
    pub fn byte_to_line(&self, byte_idx: usize) -> Option<usize> {
        (byte_idx <= self.text().len()).then(|| {
            match self
                .lines
                .binary_search_by_key(&byte_idx, |line| line.utf8_offset)
            {
                Ok(i) => i,
                Err(i) => i - 1,
            }
        })
    }

    /// Return the index of the column at the byte index.
    ///
    /// The column is defined as the number of characters in the line before the
    /// byte index.
    pub fn byte_to_column(&self, byte_idx: usize) -> Option<usize> {
        let line = self.byte_to_line(byte_idx)?;
        let start = self.line_to_byte(line)?;
        let head = self.get(start..byte_idx)?;
        Some(head.chars().count())
    }

    /// Return the byte index at the UTF-16 code unit.
    pub fn utf16_to_byte(&self, utf16_idx: usize) -> Option<usize> {
        let line = self.lines.get(
            match self
                .lines
                .binary_search_by_key(&utf16_idx, |line| line.utf16_offset)
            {
                Ok(i) => i,
                Err(i) => i - 1,
            },
        )?;

        let mut k = line.utf16_offset;
        for (i, c) in self.text()[line.utf8_offset..].char_indices() {
            if k >= utf16_idx {
                return Some(line.utf8_offset + i);
            }
            k += c.len_utf16();
        }

        (k == utf16_idx).then_some(self.text().len())
    }

    /// Return the byte position at which the given line starts.
    pub fn line_to_byte(&self, line_idx: usize) -> Option<usize> {
        self.lines.get(line_idx).map(|line| line.utf8_offset)
    }

    /// Return the range which encloses the given line.
    pub fn line_to_range(&self, line_idx: usize) -> Option<Range<usize>> {
        let start = self.line_to_byte(line_idx)?;
        let end = self.line_to_byte(line_idx + 1).unwrap_or(self.text().len());
        Some(start..end)
    }

    /// Return the byte index of the given (line, column) pair.
    ///
    /// The column defines the number of characters to go beyond the start of
    /// the line.
    pub fn line_column_to_byte(&self, line_idx: usize, column_idx: usize) -> Option<usize> {
        let range = self.line_to_range(line_idx)?;
        let line = self.get(range.clone())?;
        let mut chars = line.chars();
        for _ in 0..column_idx {
            chars.next();
        }
        Some(range.start + (line.len() - chars.as_str().len()))
    }

    /// Fully replace the source text.
    ///
    /// This performs a naive (suffix/prefix-based) diff of the old and new text
    /// to produce the smallest single edit that transforms old into new and
    /// then calls [`edit`](Self::edit) with it.
    ///
    /// Returns the range in the new source that was ultimately reparsed.
    pub fn replace(&mut self, new: &str) -> Range<usize> {
        let old = self.text();

        let mut prefix = old
            .as_bytes()
            .iter()
            .zip(new.as_bytes())
            .take_while(|(x, y)| x == y)
            .count();

        if prefix == old.len() && prefix == new.len() {
            return 0..0;
        }

        while !old.is_char_boundary(prefix) || !new.is_char_boundary(prefix) {
            prefix -= 1;
        }

        let mut suffix = old[prefix..]
            .as_bytes()
            .iter()
            .zip(new[prefix..].as_bytes())
            .rev()
            .take_while(|(x, y)| x == y)
            .count();

        while !old.is_char_boundary(old.len() - suffix) || !new.is_char_boundary(new.len() - suffix)
        {
            suffix += 1;
        }

        let replace = prefix..old.len() - suffix;
        let with = &new[prefix..new.len() - suffix];
        self.edit(replace, with)
    }

    pub fn edit(&mut self, replace: Range<usize>, with: &str) -> Range<usize> {
        let start_byte = replace.start;
        let start_utf16 = self.byte_to_utf16(start_byte).unwrap();
        let line = self.byte_to_line(start_byte).unwrap();

        //let inner = std::sync::Arc::make_mut(&mut self.0);

        // Update the text itself.
        self.tree.1.replace_range(replace.clone(), with);

        // Remove invalidated line starts.
        self.lines.truncate(line + 1);

        // Handle adjoining of \r and \n.
        if self.tree.1[..start_byte].ends_with('\r') && with.starts_with('\n') {
            self.lines.pop();
        }

        // Recalculate the line starts after the edit.
        self.lines.extend(Line::lines_from(
            start_byte,
            start_utf16,
            &self.tree.1[start_byte..],
        ));

        // Incrementally reparse the replaced range.
        self.tree.reparse(replace.clone(), with.len());
        return replace; // TODO
    }
}

/// Metadata about a line.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct Line {
    /// The UTF-8 codepoint byte offset where the line starts.
    utf8_offset: usize,
    /// The UTF-16 codepoint byte offset where the line starts.
    utf16_offset: usize,
}

impl Line {
    /// Create a line vector.
    fn lines(text: &str) -> Vec<Line> {
        std::iter::once(Line {
            utf8_offset: 0,
            utf16_offset: 0,
        })
        .chain(Line::lines_from(0, 0, text))
        .collect()
    }

    /// Compute a line iterator from an offset.
    fn lines_from(
        byte_start_offset: usize,
        utf16_start_offset: usize,
        text: &str,
    ) -> impl Iterator<Item = Line> + '_ {
        let mut utf8_offset = byte_start_offset;
        let mut utf16_offset = utf16_start_offset;
        let mut lines = Vec::new();

        text.char_indices().for_each(|(byt_off, ch)| {
            utf8_offset = byte_start_offset + byt_off;
            utf16_offset += ch.len_utf16();
            if is_newline(ch) {
                if ch == '\r' && text.get(byt_off + 1..byt_off + 2) == Some("\n") {
                    lines.push(Line {
                        utf8_offset: utf8_offset + "\r\n".len(),
                        utf16_offset: utf16_offset + '\r'.len_utf16() + '\n'.len_utf16(),
                    });
                } else if ch == '\n' && text.get(byt_off - 1..byt_off) == Some("\r") {
                    // added on previous iteration
                } else {
                    lines.push(Line {
                        utf8_offset: utf8_offset + ch.len_utf8(),
                        utf16_offset: utf16_offset + ch.len_utf8(),
                    });
                }
            }
        });
        return lines.into_iter();
    }
}

/// The number of code units this string would use if it was encoded in
/// UTF16. This runs in linear time.
fn len_utf16(string: &str) -> usize {
    string.chars().map(char::len_utf16).sum()
}

pub fn is_newline(c: char) -> bool {
    return c == '\n'         // line feed
        || c == '\x0B'       // vertical tab
        || c == '\x0C'       // form feed
        || c == '\r'         // carriage return
        || c == '\u{0085}'   // next line
        || c == '\u{2028}'   // line seperator
        || c == '\u{2029}'; // paragraph seperator
}
