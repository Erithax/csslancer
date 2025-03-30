/// TAKEN FROM THE RUST LANGUAGE COMPILER LEXER
/// rust-lang/rust/compiler/rustc-lexer/src/cursor.rs
// TODO: perf repeated iter.next() vs iter.nth()

use std::str::Chars;

use miette::Diagnostic;
use thiserror::Error;

// TODO? : feature gate miette
#[derive(Debug, Error, Diagnostic)]
pub enum LexerDiagnostic {
    #[error("Oops it blew up")]
    OutOfNonSurrogateValidUtf8RangeEscapedCodePoint,
    #[error("Uh no!")]
    EscapedCodePointEncounteredEof,
    #[error("Unexpected solidus (forward slash)")]
    UnexpectedSolidus,
    #[error("Unterminated comment went into end of file")]
    UnterminatedCommentFoundEof,
    #[error("Unterminated string went into end of file")]
    UnterminatedStringFoundEof,
    #[error("Newline in string")]
    NewlineInString,
    #[error("Unterminated url went into end of file")]
    UnterminatedUrlFoundEof,
    #[error("Unexpected character in url")]
    UnexpectedCharInUrl,
}

pub struct PosedLexerDiagnostic {
    pub diagnostic: LexerDiagnostic,
    pub token_idx: u32,
}

/// Peekable iterator over a char sequence.
///
/// Next characters can be peeked via `first` method,
/// and position can be shifted forward via `bump` method.
pub struct Cursor<'a> {
    len_remaining: usize,
    /// Iterator over chars. Slightly faster than a &str.
    chars: Chars<'a>,
    // TODO? : feature gate miette
    /// Next token will have index `token_idx`
    pub(crate) token_idx: usize,
    diagnostics: Vec<PosedLexerDiagnostic>,   
    #[cfg(debug_assertions)]
    prev: char,
}

pub(crate) const EOF_CHAR: char = '\0';

impl<'a> Cursor<'a> {

    pub fn new(input: &'a str) -> Cursor<'a> {
        Cursor {
            len_remaining: input.len(),
            chars: input.chars(),
            token_idx: 0,
            diagnostics: Vec::new(),
            #[cfg(debug_assertions)]
            prev: EOF_CHAR,
        }
    }

    pub fn emit_diagnostic_for_curr(&mut self, diagnostic: LexerDiagnostic) {
        self.diagnostics.push(PosedLexerDiagnostic {diagnostic, token_idx: self.token_idx as u32});
    }

    pub fn emit_diagnostic_for_next(&mut self, diagnostic: LexerDiagnostic) {
        self.diagnostics.push(PosedLexerDiagnostic {diagnostic, token_idx: self.token_idx as u32 + 1});
    }

    // pub fn as_str(&self) -> &'a str {
    //     self.chars.as_str()
    // }

    /// Returns the last eaten symbol (or `'\0'` in release builds).
    /// (For debug assertions only.)
    // pub(crate) fn prev(&self) -> char {
    //     #[cfg(debug_assertions)]
    //     {
    //         self.prev
    //     }

    //     #[cfg(not(debug_assertions))]
    //     {
    //         EOF_CHAR
    //     }
    // }

    pub fn take_diagnostics(&mut self) -> Vec<PosedLexerDiagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    pub fn mut_diagnostics(&mut self) -> &mut Vec<PosedLexerDiagnostic> {
        &mut self.diagnostics
    }

    /// Peeks the next symbol from the input stream without consuming it.
    /// If requested position doesn't exist, `EOF_CHAR` is returned.
    /// However, getting `EOF_CHAR` doesn't always mean actual end of file,
    /// it should be checked with `is_eof` method.
    pub(crate) fn first(&self) -> char {
        self.chars.clone().next().unwrap_or(EOF_CHAR)
    }

    pub(crate) fn second(&self) -> char {
        let mut iter = self.chars.clone();
        iter.next();
        iter.next().unwrap_or(EOF_CHAR)
    }

    pub(crate) fn third(&self) -> char {
        let mut iter = self.chars.clone();
        iter.next();
        iter.next();
        iter.next().unwrap_or(EOF_CHAR)
    }

    pub(crate) fn fourth(&self) -> char {
        let mut iter = self.chars.clone();
        iter.next();
        iter.next();
        iter.next();
        iter.next().unwrap_or(EOF_CHAR)
    }

    // pub(crate) fn peek_nth(&self, i: usize) -> char {
    //     assert!(
    //     // reconsumption code
    //     if let Some(c) = self.holdbox {
    //         if i == 0 {

    //         }
    //     }
    //     // ---
    //     self.chars.clone().nth(i).unwrap_or(EOF_CHAR)
    // }

    // pub(crate) fn starts_with(&self, s: &str) -> bool {
    //     let mut self_chars_clone = s.chars().clone();
    //     s.chars().all(|ch| Some(ch) == self_chars_clone.next())
    // }

    /// https://drafts.csswg.org/css-syntax/#charset-rule
    /// https://drafts.csswg.org/css-syntax/#determine-the-fallback-encoding
    /// Maybe consume charset rule and return `true` if we did
    pub(crate) fn maybe_consume_charset(&mut self) -> bool {
        // do not consume anything until full match is ensured
        let mut self_chars_clone = self.chars.clone();
        "@charset \"".chars().all(|ch| Some(ch) == self_chars_clone.next());
        
        let mut success = false;
        let mut encoding_chars_count = 0;
        for ch in self_chars_clone.by_ref() {
            if ch == '"' {
                success = true;
                break
            } else if !ch.is_ascii() {
                return false
            } else {
                encoding_chars_count += 1;
            }
        }

        if !success {
            return false
        }

        if Some(';') != self_chars_clone.next() {
            return false
        }
        self.bump_n("@charset \"\";".len() + encoding_chars_count);
        true
    }

    /// Checks if there is nothing more to consume.
    pub(crate) fn is_eof(&self) -> bool {
        self.chars.as_str().is_empty()
    }

    /// Returns amount of already consumed symbols.
    pub(crate) fn pos_within_token(&self) -> u32 {
        (self.len_remaining - self.chars.as_str().len()) as u32
    }

    /// Resets the number of bytes consumed to 0.
    pub(crate) fn reset_pos_within_token(&mut self) {
        self.len_remaining = self.chars.as_str().len();
    }

    /// Moves to the next character.
    pub(crate) fn bump(&mut self) -> Option<char> {
        let c = self.chars.next()?;

        #[cfg(debug_assertions)]
        {
            self.prev = c;
        }

        Some(c)
    }

    pub(crate) fn bump_two(&mut self) -> Option<char> {
        self.chars.next()?;
        let c = self.chars.next()?;

        #[cfg(debug_assertions)]
        {
            self.prev = c;
        }

        Some(c)
    }

    pub(crate) fn bump_three(&mut self) -> Option<char> {
        self.chars.next()?;
        self.chars.next()?;
        let c = self.chars.next()?;

        #[cfg(debug_assertions)]
        {
            self.prev = c;
        }

        Some(c)
    }

    pub(crate) fn bump_four(&mut self) -> Option<char> {
        self.chars.next()?;
        self.chars.next()?;
        self.chars.next()?;
        let c = self.chars.next()?;
        
        #[cfg(debug_assertions)]
        {
            self.prev = c;
        }

        Some(c)
    }

    pub(crate) fn bump_n(&mut self, n: usize) -> Option<char> {
        if n != 0 {
            return self.chars.nth(n-1)
        }
        None
    }

    /// Eats symbols while predicate returns true or until the end of file is reached.
    pub(crate) fn bump_while_first(&mut self, mut predicate: impl FnMut(char) -> bool) {
        // It was tried making optimized version of this for eg. line comments, but
        // LLVM can inline all of this and compile it down to fast iteration over bytes.
        while predicate(self.first()) && !self.is_eof() {
            self.bump();
        }
    }
}