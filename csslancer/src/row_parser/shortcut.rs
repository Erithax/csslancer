//! Shortcuts that span lexer/parser abstraction.
//!
//! The way Rust works, parser doesn't necessary parse text, and you might
//! tokenize text without parsing it further. So, it makes sense to keep
//! abstract token parsing, and string tokenization as completely separate
//! layers.
//!
//! However, often you do pares text into syntax trees and the glue code for
//! that needs to live somewhere. Rather than putting it to lexer or parser, we
//! use a separate shortcuts module for that.

use std::mem;

use crate::T;

use super::{
    lex_to_syn::LexedStr, 
    output::Step,
    syntax_kind_gen::SyntaxKind::{self, *},
};

#[derive(Debug)]
pub enum StrStep<'a> {
    Token { kind: SyntaxKind, text: &'a str },
    Enter { kind: SyntaxKind },
    Exit,
    Error { msg: &'a str, pos: usize },
}

fn cx_hash(s: &str) -> Option<SyntaxKind> {
    let mut chars = s.chars();
    match chars.next() {
        Some('#') => {},
        None => return None,
        _ => return None,
    };
    let mut count = 0;
    for c in chars {
        if !c.is_ascii_hexdigit() {   
            return None
        }
        count += 1;
    }
    matches!(count, 3 | 4 | 6 | 8).then_some(SyntaxKind::CXHASH_VALID_HEX)
}

fn cx_func(s: &str) -> Option<SyntaxKind> {
    match s.to_lowercase().as_str() {
        "style(" => Some(SyntaxKind::CXFUNC_STYLE),
        "url(" => Some(SyntaxKind::CXFUNC_URL),
        "layer(" => Some(SyntaxKind::CXFUNC_LAYER),
        "supports(" => Some(SyntaxKind::CXFUNC_SUPPORTS),
        _ => None
    }
}

fn cx_id(s: &str) -> Option<SyntaxKind> {
    // "not",
    // "and",
    // "or",
    // "only",
    // "deep",
    // "attrib_i",
    // "attrib_s",
    // "an_plus_b_syntax_an",
    // "of",
    // "important",
    // "progid",
    // "url",
    // "urlprefix",
    // "valid_custom_prop",
    match s.to_lowercase().as_str() {
        "not" => Some(T![cxid_not]),
        "and" => Some(T![cxid_and]),
        "or" => Some(T![cxid_or]),
        "only" => Some(T![cxid_only]),
        "deep" => Some(T![cxid_deep]),
        "i" => Some(T![cxid_attrib_i]),
        "s" => Some(T![cxid_attrib_s]),
        "n" | "-n" => Some(T![cxid_an_plus_b_syntax_an]),
        "of" => Some(T![cxid_of]),
        "important" => Some(T![cxid_important]),
        "progid" => Some(T![cxid_progid]),
        "urlprefix" => Some(T![cxid_urlprefix]),
        "valid_custom_prop" => Some(T![cxid_valid_custom_prop]),
        s if {
            let mut chars = s.chars();
            chars.next().is_some_and(|c| c == '-') && 
                chars.next().is_some_and(|c| c == '-')
        } => {Some(T![cxid_valid_custom_prop])},
        _ => None
    }
}

fn cx_dim(s: &str) -> Option<SyntaxKind> {
    if 
        !s.chars().last().is_some_and(|ch| ch == 'n') ||
        !(
            s.len() == 1 || 
            s[0..s.len()-1].chars().all(|ch| ch.is_ascii_digit())
        )
    {
        return None
    }
    Some(SyntaxKind::CXDIM_AN_PLUS_B)
}



impl LexedStr<'_> {
    pub fn to_input(&self) -> super::input::Input {
        let _p = tracing::span!(tracing::Level::INFO, "LexedStr::to_input").entered();
        let mut res = super::input::Input::default();
        let mut set_had_whitespace = true; // initial value of `true` means that starting trivia will be consumed without calling .had_whitespace()

        for i in 0..self.len() {
            let kind = self.kind(i);
            if kind.is_trivia() {
                if !set_had_whitespace {
                    res.had_whitespace();
                    set_had_whitespace = true;
                }
                continue
            } 
            set_had_whitespace = false;
            if kind == SyntaxKind::IDENTIFIER {
                let contextual_kw = cx_id(self.text(i)).unwrap_or(SyntaxKind::IDENTIFIER);
                res.push_ident(contextual_kw);
            } else if kind == SyntaxKind::FUNCTION {
                let contextual_kw = cx_func(self.text(i)).unwrap_or(SyntaxKind::FUNCTION);
                res.push_func(contextual_kw);
            } else if kind == SyntaxKind::ID_HASH {
                let contextual_kw = cx_hash(self.text(i)).unwrap_or(SyntaxKind::ID_HASH);
                res.push_id_hash(contextual_kw);
            } else if kind == SyntaxKind::UNRESTRICTED_HASH {
                let contextual_kw = cx_hash(self.text(i)).unwrap_or(SyntaxKind::UNRESTRICTED_HASH);
                res.push_unrestriced_hash(contextual_kw);
            } else if kind == SyntaxKind::DIM_UNKNOWN {
                let contextual_kw = cx_dim(self.text(i)).unwrap_or(SyntaxKind::DIM_UNKNOWN);
                res.push_unknown_dimension(contextual_kw);
            } else {
                res.push(kind);
            }
        }
        res
    }

    /// NB: only valid to call with Output from Reparser/TopLevelEntry.
    pub fn intersperse_trivia(
        &self,
        output: &super::output::Output,
        sink: &mut dyn FnMut(StrStep<'_>),
    ) -> bool {
        let mut builder = Builder { lexed: self, pos: 0, state: State::PendingEnter, sink };

        for event in output.iter() {
            match event {
                Step::Token { kind, n_input_tokens: n_raw_tokens } => {
                    builder.token(kind, n_raw_tokens)
                },
                Step::Enter { kind } => builder.enter(kind),
                Step::Exit => builder.exit(),
                Step::Error { msg } => {
                    let text_pos = builder.lexed.text_start(builder.pos);
                    (builder.sink)(StrStep::Error { msg, pos: text_pos });
                }
            }
        }

        match mem::replace(&mut builder.state, State::Normal) {
            State::PendingExit => {
                builder.eat_trivias();
                (builder.sink)(StrStep::Exit);
            }
            State::PendingEnter | State::Normal => unreachable!(),
        }

        // is_eof?
        builder.pos == builder.lexed.len()
    }
}

struct Builder<'a, 'b> {
    lexed: &'a LexedStr<'a>,
    pos: usize,
    state: State,
    sink: &'b mut dyn FnMut(StrStep<'_>),
}

enum State {
    PendingEnter,
    Normal,
    PendingExit,
}

impl Builder<'_, '_> {
    fn token(&mut self, kind: SyntaxKind, n_tokens: u8) {
        match mem::replace(&mut self.state, State::Normal) {
            State::PendingEnter => unreachable!(),
            State::PendingExit => (self.sink)(StrStep::Exit),
            State::Normal => (),
        }
        self.eat_trivias();
        self.do_token(kind, n_tokens as usize);
    }

    fn enter(&mut self, kind: SyntaxKind) {
        match mem::replace(&mut self.state, State::Normal) {
            State::PendingEnter => {
                (self.sink)(StrStep::Enter { kind });
                // No need to attach trivias to previous node: there is no
                // previous node.
                return;
            }
            State::PendingExit => (self.sink)(StrStep::Exit),
            State::Normal => (),
        }

        let n_trivias =
            (self.pos..self.lexed.len()).take_while(|&it| self.lexed.kind(it).is_trivia()).count();
        let leading_trivias = self.pos..self.pos + n_trivias;
        let n_attached_trivias = n_attached_trivias(
            kind,
            leading_trivias.rev().map(|it| (self.lexed.kind(it), self.lexed.text(it))),
        );
        self.eat_n_trivias(n_trivias - n_attached_trivias);
        (self.sink)(StrStep::Enter { kind });
        self.eat_n_trivias(n_attached_trivias);
    }

    fn exit(&mut self) {
        match mem::replace(&mut self.state, State::PendingExit) {
            State::PendingEnter => unreachable!(),
            State::PendingExit => (self.sink)(StrStep::Exit),
            State::Normal => (),
        }
    }

    fn eat_trivias(&mut self) {
        while self.pos < self.lexed.len() {
            let kind = self.lexed.kind(self.pos);
            if !kind.is_trivia() {
                break;
            }
            self.do_token(kind, 1);
        }
    }

    fn eat_n_trivias(&mut self, n: usize) {
        for _ in 0..n {
            let kind = self.lexed.kind(self.pos);
            assert!(kind.is_trivia());
            self.do_token(kind, 1);
        }
    }

    fn do_token(&mut self, kind: SyntaxKind, n_tokens: usize) {
        let text = &self.lexed.range_text(self.pos..self.pos + n_tokens);
        self.pos += n_tokens;
        (self.sink)(StrStep::Token { kind, text });
    }
}

fn n_attached_trivias<'a>(
    kind: SyntaxKind,
    trivias: impl Iterator<Item = (SyntaxKind, &'a str)>,
) -> usize {
    match kind {
        TODO => {
            let mut res = 0;
            let mut trivias = trivias.enumerate().peekable();

            while let Some((i, (kind, text))) = trivias.next() {
                match kind {
                    WHITESPACE if text.contains("\n\n") => {
                        // we check whether the next token is a doc-comment
                        // and skip the whitespace in this case
                        if let Some((COMMENT, peek_text)) = trivias.peek().map(|(_, pair)| pair) {
                            if is_outer(peek_text) {
                                continue;
                            }
                        }
                        break;
                    }
                    COMMENT => {
                        if is_inner(text) {
                            break;
                        }
                        res = i + 1;
                    }
                    _ => (),
                }
            }
            res
        }
        _ => 0,
    }
}

fn is_outer(text: &str) -> bool {
    if text.starts_with("////") || text.starts_with("/***") {
        return false;
    }
    text.starts_with("///") || text.starts_with("/**")
}

fn is_inner(text: &str) -> bool {
    text.starts_with("//!") || text.starts_with("/*!")
}
