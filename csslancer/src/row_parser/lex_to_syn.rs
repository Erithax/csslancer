

use std::fmt::Debug;
use std::ops;

use crate::tokenizer::cursor::PosedLexerDiagnostic;
use crate::tokenizer::{tokenize_file, TokenKind};
use crate::T;
use super::syntax_kind_gen::SyntaxKind;

pub struct LexedStr<'a> {
    text: &'a str,
    kind: Vec<SyntaxKind>,
    start: Vec<u32>,
    error: Vec<PosedLexerDiagnostic>,
}

impl<'a> LexedStr<'a> {
    pub fn new(text: &'a str) -> LexedStr<'a> {
        let _p = tracing::span!(tracing::Level::INFO, "LexedStr::new").entered();
        let mut conv = Converter::new(text);
        // if let Some(shebang_len) = strip_shebang(text) {
        //     conv.res.push(SHEBANG, conv.offset);
        //     conv.offset = shebang_len;
        // };

        let mut diags = Vec::new();
        
        let mut tok_count = 0;
        for token in tokenize_file(text, &mut diags) {
            tok_count += 1;
            let token_text = &text[conv.offset..][..token.len as usize];
            conv.extend_token(token.kind, token_text);
        }

        let mut do_panic = false;
        let l = conv.res.kind.len() as u32;
        assert_eq!(tok_count, conv.res.kind.len());
        for diag in diags.iter() {
            if diag.token_idx >= l {
                do_panic = true;
                println!("diag token idx `{}` out of range `{}` : `{}`", diag.token_idx, l, diag.diagnostic.to_string());
            }
        }
        if do_panic {
            println!("No Bueno {:?}", conv.res.kind.iter().fold(String::new(), |acc, nex| acc + " > " + &format!("{:?}", nex)));
        }

        let res = conv.finalize_with_eof_and_errs(diags);

        res
    }

    pub fn single_token(text: &'a str) -> Option<(SyntaxKind, impl IntoIterator<Item = String>)> {
        if text.is_empty() {
            return None;
        }
        let mut diags = Vec::new();
        let mut tokens = tokenize_file(text, &mut diags);
        let token = tokens.next()?;

        // have to call next again to write the errors
        if token.len as usize != text.len() || tokens.next().is_some() {
            return None;
        }
        drop(tokens);
        let mut conv = Converter::new(text);
        conv.extend_token(token.kind, text);
        match &*conv.res.kind {
            [kind] => {
                //Some((*kind, conv.res.error.into_iter().map(|it| it.diagnostic.to_string())))
                Some((*kind, diags.into_iter().map(|diag| diag.diagnostic.to_string())))
            }
            _ => None,
        }
    }

    pub fn as_str(&self) -> &str {
        self.text
    }

    pub fn len(&self) -> usize {
        self.kind.len() - 1
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn kind(&self, i: usize) -> SyntaxKind {
        assert!(i < self.len());
        self.kind[i]
    }

    pub fn text(&self, i: usize) -> &str {
        self.range_text(i..i + 1)
    }

    pub fn range_text(&self, r: ops::Range<usize>) -> &str {
        assert!(r.start < r.end && r.end <= self.len());
        let lo = self.start[r.start] as usize;
        let hi = self.start[r.end] as usize;
        &self.text[lo..hi]
    }

    // Naming is hard.
    pub fn text_range(&self, i: usize) -> ops::Range<usize> {
        assert!(i < self.len(), "tried to get text range of token idx `{}` out of range (len = {})", i, self.len());
        let lo = self.start[i] as usize;
        let hi = self.start[i + 1] as usize;
        lo..hi
    }
    pub fn text_start(&self, i: usize) -> usize {
        assert!(i <= self.len());
        self.start[i] as usize
    }
    pub fn text_len(&self, i: usize) -> usize {
        assert!(i < self.len());
        let r = self.text_range(i);
        r.end - r.start
    }

    // FIXME: return slice of errors
    pub fn error(&self, i: usize) -> Option<String> {
        assert!(i < self.len());
        let err = self.error.binary_search_by_key(&(i as u32), |i| i.token_idx).ok()?;
        Some(self.error[err].diagnostic.to_string())
    }

    pub fn errors(&self) -> impl Iterator<Item = (usize, String)> + '_ {
        self.error.iter().map(|it| (it.token_idx as usize, it.diagnostic.to_string()))
    }

    fn push(&mut self, kind: SyntaxKind, offset: usize) {
        self.kind.push(kind);
        self.start.push(offset as u32);
    }
}

struct Converter<'a> {
    pub res: LexedStr<'a>,
    offset: usize,
}

impl<'a> Converter<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            res: LexedStr { text, kind: Vec::new(), start: Vec::new(), error: Vec::new() },
            offset: 0,
        }
    }

    fn finalize_with_eof_and_errs(mut self, diagnostics: Vec<PosedLexerDiagnostic>) -> LexedStr<'a> {
        self.res.push(SyntaxKind::EOF, self.offset);
        self.res.error = diagnostics;
        self.res
    }

    fn push(&mut self, kind: SyntaxKind, len: usize) {
        self.res.push(kind, self.offset);
        self.offset += len;
    }

    fn extend_token(&mut self, kind: TokenKind, token_text: &str) {
        // A note on an intended tradeoff:
        // We drop some useful information here (see patterns with double dots `..`)
        // Storing that info in `SyntaxKind` is not possible due to its layout requirements of
        // being `u16` that come from `rowan::SyntaxKind`.

        let syntax_kind = {
            match kind {
                TokenKind::WhiteSpace => T![whitespace],
                TokenKind::Comment => T![comment],
                TokenKind::Charset => T![charset],
                TokenKind::String => T![string],
                TokenKind::Url => T![url],
                TokenKind::Number => T![number],
                TokenKind::UnicodeRange => T![unicode_range],
                TokenKind::BadUrl => T![bad_url],
                TokenKind::BadString => T![bad_string],
                TokenKind::UnrestrictedHash => T![unrestricted_hash],
                TokenKind::IdHash => T![id_hash],
                TokenKind::Ident => T![identifier],
                TokenKind::CDO => T![cdo],
                TokenKind::CDC => T![cdc],
                TokenKind::Function => T![function],
                TokenKind::AtKeyword => {
                    match token_text.to_lowercase().as_str() {
                        "@import" => SyntaxKind::ATKW_IMPORT,
                        "@namespace" => SyntaxKind::ATKW_NAMESPACE,
                        "@font-face" => SyntaxKind::ATKW_FONT_FACE,
                        "@viewport" => SyntaxKind::ATKW_VIEWPORT,
                        "@-ms-viewport" => SyntaxKind::ATKW__MS_VIEWPORT,
                        "@-o-viewport" => SyntaxKind::ATKW__O_VIEWPORT,
                        "@keyframes" => SyntaxKind::ATKW_KEYFRAMES,
                        "@-webkit-keyframes" => SyntaxKind::ATKW__WEBKIT_KEYFRAMES,
                        "@-moz-keyframes" => SyntaxKind::ATKW__MOZ_KEYFRAMES,
                        "@-o-keyframes" => SyntaxKind::ATKW__O_KEYFRAMES,
                        "@property" => SyntaxKind::ATKW_PROPERTY,
                        "@layer" => SyntaxKind::ATKW_LAYER,
                        "@supports" => SyntaxKind::ATKW_SUPPORTS,
                        "@media" => SyntaxKind::ATKW_MEDIA,
                        "@page" => SyntaxKind::ATKW_PAGE,
                        "@-moz-document" => SyntaxKind::ATKW__MOZ_DOCUMENT,
                        "@container" => SyntaxKind::ATKW_CONTAINER,

                        // https://developer.mozilla.org/en-US/docs/Web/CSS/@page#margin_at-rules
                        "@top-left-corner" |
                        "@top-left" |
                        "@top-center" |
                        "@top-right" |
                        "@top-right-corner" |
                        "@bottom-left-corner" |
                        "@bottom-left" |
                        "@bottom-center" |
                        "@bottom-right" |
                        "@bottom-right-corner" |
                        "@left-top" |
                        "@left-middle" |
                        "@left-bottom" |
                        "@right-top" |
                        "@right-middle" |
                        "@right-bottom" => SyntaxKind::ATKW_MARGIN_AT_RULE,
                        
                        _ => SyntaxKind::ATKW_UNKNOWN,
                    }
                },
                TokenKind::Dimension => {
                    match token_text.to_lowercase().as_str() {
                        s if s.ends_with("em") => T![DIM_EM],
                        s if s.ends_with("ex") => T![DIM_EX],
                        s if s.ends_with("px") => T![DIM_PX],
                        s if s.ends_with("cm") => T![DIM_CM],
                        s if s.ends_with("mm") => T![DIM_MM],
                        s if s.ends_with("in") => T![DIM_IN],
                        s if s.ends_with("pt") => T![DIM_PT],
                        s if s.ends_with("pc") => T![DIM_PC],
                        s if s.ends_with("deg") => T![DIM_DEG],
                        s if s.ends_with("rad") => T![DIM_RAD],
                        s if s.ends_with("grad") => T![DIM_GRAD],
                        s if s.ends_with("ms") => T![DIM_MS],
                        s if s.ends_with('s') => T![DIM_S],
                        s if s.ends_with("hz") => T![DIM_HZ],
                        s if s.ends_with("khz") => T![DIM_KHZ],
                        s if s.ends_with("fr") => T![DIM_FR],
                        s if s.ends_with("dpi") => T![DIM_DPI],
                        s if s.ends_with("dpcm") => T![DIM_DPCM],
                        s if s.ends_with("cqw") => T![DIM_CQW],
                        s if s.ends_with("cqh") => T![DIM_CQH],
                        s if s.ends_with("cqi") => T![DIM_CQI],
                        s if s.ends_with("cqb") => T![DIM_CQB],
                        s if s.ends_with("cqmin") => T![DIM_CQMIN],
                        s if s.ends_with("cqmax") => T![DIM_CQMAX],
                        _ => T![DIM_UNKNOWN],
                    }
                },
                TokenKind::Percentage => T![DIM_PERCENT],
                TokenKind::Colon => T![:],
                TokenKind::Semicolon => T![;],
                TokenKind::Comma => T![,],
                TokenKind::OpenParen => SyntaxKind::L_PAREN,
                TokenKind::CloseParen => SyntaxKind::R_PAREN,
                TokenKind::OpenBracket => SyntaxKind::L_BRACK,
                TokenKind::CloseBracket => SyntaxKind::R_BRACK,
                TokenKind::OpenCurly => SyntaxKind::L_CURLY,
                TokenKind::CloseCurly => SyntaxKind::R_CURLY,
                TokenKind::DelimSlash => T![/],
                TokenKind::DelimHash => T![#],
                TokenKind::DelimPlus => T![+],
                TokenKind::DelimHyphenMinus => T![-],
                TokenKind::DelimFullStop => T![.],
                TokenKind::DelimLessThanSign => T![<],
                TokenKind::DelimGreaterThanSign => T![>],
                TokenKind::DelimCommercialAt => T![@],
                TokenKind::DelimExclamation => T![!],
                TokenKind::Unknown => {
                    match token_text {
                        "=" => T![=],
                        "$" => SyntaxKind::DOLLAR,
                        "~" => T![~],
                        "|" => T![|],
                        "*" => T![*],
                        "^" => T![^],
                        "&" => T![&],
                        "?" => T![?],
                        _ => T![error]
                    }
                },
                TokenKind::DelimUnexpectedSolidus => T![error],

                TokenKind::Eof => SyntaxKind::EOF,
            }
        };

        self.push(syntax_kind, token_text.len());
    }

}