//! Source text with the position conversions an editor needs.
//!
//! Rust slices strings by UTF-8 byte offset, but JavaScript editors address text by UTF-16 code unit.
//! A multi-byte character ("é", an emoji) makes the two disagree, so every span that leaves this
//! crate is converted here instead of being handed over as bytes.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Byte offsets into the source, `start <= end`, always on character boundaries.
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        debug_assert!(start <= end);
        Span { start, end }
    }
    pub fn join(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// An editor-facing position: zero-based line and column, plus a UTF-16 offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    /// Column in UTF-16 code units from the start of the line.
    pub column: usize,
    /// Offset in UTF-16 code units from the start of the text.
    pub offset: usize,
}

pub struct Source<'a> {
    pub text: &'a str,
    /// `utf16[i]` is the UTF-16 offset of byte `i` (for every character boundary; other entries are
    /// the offset of the character they fall inside).
    utf16: Vec<u32>,
    line_starts: Vec<usize>,
}

impl<'a> Source<'a> {
    pub fn new(text: &'a str) -> Self {
        let mut utf16 = vec![0u32; text.len() + 1];
        let mut line_starts = vec![0usize];
        let mut unit = 0u32;
        for (i, ch) in text.char_indices() {
            for slot in utf16.iter_mut().skip(i).take(ch.len_utf8()) {
                *slot = unit;
            }
            unit += ch.len_utf16() as u32;
            if ch == '\n' {
                line_starts.push(i + 1);
            }
        }
        utf16[text.len()] = unit;
        Source {
            text,
            utf16,
            line_starts,
        }
    }

    pub fn position(&self, byte: usize) -> Position {
        let byte = byte.min(self.text.len());
        let line = match self.line_starts.binary_search(&byte) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let offset = self.utf16[byte] as usize;
        let column = offset - self.utf16[self.line_starts[line]] as usize;
        Position {
            line,
            column,
            offset,
        }
    }

    pub fn slice(&self, span: Span) -> &'a str {
        &self.text[span.start..span.end]
    }
}
