/// A css lexer based on [CSS Syntax Module Level 3](https://drafts.csswg.org/css-syntax/)
/// with some notable differences: 
/// - tokens with flags are flattened into the below `TokenKind` (e.g. <hash-token> becomes IdHash or UnrestrictedHash)
/// - comments are kept
/// - the output of many consumers of the [CSS tokenizer spec](https://drafts.csswg.org/css-syntax/#tokenization) is 
///     not simply a `Token` with a kind and length like here
///     we defer this to the parser as opposed to the lexer
/// - TODO:? different behaviour on some parsing errors
/// - TODO:? extra token kinds like [VSCode CSS Language Service](https://github.com/microsoft/vscode-css-languageservice)

mod cursor;
mod test;

use cursor::Cursor;
use cursor::EOF_CHAR;

/// Parsed token.
#[derive(Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub len: u32,
}

impl Token {
    pub fn new(kind: TokenKind, len: u32) -> Self {
        Self {
            kind,
            len,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    // ////////////////////////////////////////////////////////////////////////////////////////////////////////////
    // CSS Syntax Spec tokens
    // ////////////////////////////////////////////////////////////////////////////////////////////////////////////

    /// [`<whitespace-token>`](https://drafts.csswg.org/css-syntax/#whitespace-token-diagram)
    WhiteSpace,

    /// [`<ident-token>`](https://drafts.csswg.org/css-syntax/#ident-token-diagram)
    Ident,

    /// [`<function-token>`](https://drafts.csswg.org/css-syntax/#function-token-diagram)
    ///
    /// The value (name) does not include the `(` marker.
    Function,

    /// [`<at-keyword-token>`](https://drafts.csswg.org/css-syntax/#at-keyword-token-diagram)
    ///
    /// The value does not include the `@` marker.
    AtKeyword,

    /// [`<hash-token>`](https://drafts.csswg.org/css-syntax/#hash-token-diagram) with the type flag set to "unrestricted"
    ///
    /// The value does not include the `#` marker.
    UnrestrictedHash,

    /// [`<hash-token>`](https://drafts.csswg.org/css-syntax/#hash-token-diagram) with the type flag set to "id"
    ///
    /// The value does not include the `#` marker.
    /// Hash that is a valid ID selector.
    IdHash, 

    /// [`<string-token>`](https://drafts.csswg.org/css-syntax/#string-token-diagram)
    ///
    /// The value does not include the quotes.
    String,

    /// [`<url-token>`](https://drafts.csswg.org/css-syntax/#url-token-diagram)
    ///
    /// The value does not include the `url(` `)` markers.  Note that `url( <string-token> )` is represented by a
    /// `Function` token.
    Url,

    /// [`<number-token>`](https://drafts.csswg.org/css-syntax/#number-token-diagram)
    Number,

    /// [`<dimension-token>`](https://drafts.csswg.org/css-syntax/#dimension-token-diagram)
    Dimension,

    /// [`<percentage-token>`](https://drafts.csswg.org/css-syntax/#percentage-token-diagram)
    Percentage,

    /// `<!--` [`<CDO-token>`](https://drafts.csswg.org/css-syntax/#CDO-token-diagram)
    CDO,

    /// `-->` [`<CDC-token>`](https://drafts.csswg.org/css-syntax/#CDC-token-diagram)
    CDC,

    /// [`<unicode-range-token>`](https://drafts.csswg.org/css-syntax/#unicode-range-token-diagram)
    UnicodeRange,

    
    // [`<delim-token>`](https://drafts.csswg.org/css-syntax/#typedef-delim-token)
    // Delim,
    // Delim token is flattened into token kinds for each known (occurs in spec) delimiter token
    // See below

    /// `:` `<colon-token>`
    Colon, // :
    /// `;` `<semicolon-token>`
    Semicolon, // ;
    /// `,` `<comma-token>`
    Comma, // ,
    /// `<(-token>`
    OpenParen,
    /// `<)-token>`
    CloseParen,
    /// `<[-token>`
    OpenBracket,
    /// `<]-token>`
    CloseBracket,
    /// `<{-token>`
    OpenCurly,
    /// `<}-token>`
    CloseCurly,

    /// [`<bad-url-token>`](https://drafts.csswg.org/css-syntax/#typedef-bad-url-token)
    ///
    /// This token always indicates a parse error.
    BadUrl,

    /// [`<bad-string-token>`](https://drafts.csswg.org/css-syntax/#typedef-bad-string-token)
    ///
    /// This token always indicates a parse error.
    BadString,

    // DELIMS FLATTENED

    DelimSlash,
    DelimHash,
    DelimPlus,
    DelimHyphenMinus,
    DelimFullStop,
    DelimLessThanSign,
    DelimGreaterThanSign,
    DelimCommercialAt,
    DelimExclamation,
    Unknown,

    DelimUnexpectedSolidus,

    // ////////////////////////////////////////////////////////////////////////////////////////////////////////////
    // Additional tokens
    // ////////////////////////////////////////////////////////////////////////////////////////////////////////////


    /// The value does not include the `/*` `*/` markers.
    Comment,
    Charset,


    /// End of input
    Eof,

    // ////////////////////////////////////////////////////////////////////////////////////////////////////////////
    // More specific VSCode CSS language Service tokens
    // ////////////////////////////////////////////////////////////////////////////////////////////////////////////

    // EMS,
    // EXS,
    // Length,
    // Angle,
    // Time,
    // Freq,
    // Resolution,
    // EscapedJavascript,
    // BadEscapedJavascript,
    // SingleLineComment,
    // ContainerQueryLength,
    // CustomToken,

    // FROM EARLIER CSS SPEC
    // /// `~=` [`<include-match-token>`](https://drafts.csswg.org/css-syntax/#include-match-token-diagram)
    // IncludeMatch,

    // /// `|=` [`<dash-match-token>`](https://drafts.csswg.org/css-syntax/#dash-match-token-diagram)
    // DashMatch,

    // /// `^=` [`<prefix-match-token>`](https://drafts.csswg.org/css-syntax/#prefix-match-token-diagram)
    // PrefixMatch,

    // /// `$=` [`<suffix-match-token>`](https://drafts.csswg.org/css-syntax/#suffix-match-token-diagram)
    // SuffixMatch,

    // /// `*=` [`<substring-match-token>`](https://drafts.csswg.org/css-syntax/#substring-match-token-diagram)
    // SubstringMatch,
}

/// Tokenizes the input string slice. 
/// If your input is a slice from 0..X of a CSS source file,
/// you should use `tokenize_file` instead as this allows `@charset-rules` 
/// at the very start of the input.
pub fn tokenize(input: &str) -> impl Iterator<Item = Token> + '_ {
    let mut cursor = Cursor::new(input);
    std::iter::from_fn(move || {
        let token = cursor.consume_token();
        if token.kind != TokenKind::Eof {Some(token) } else { None }
    })
}

/// Like `tokenize`, except it allows a `@charset-rule` at the very start of the input.
pub fn tokenize_file(input: &str) -> impl Iterator<Item = Token> + '_ {
    let mut cursor = Cursor::new(input);
    let charset = cursor.maybe_consume_charset();
    let first = if charset {
        Token {
            kind: TokenKind::Charset,
            len: cursor.pos_within_token(),
        }
    } else {
        cursor.consume_token()
    };

    let first_is_eof = first.kind == TokenKind::Eof;

    let consumer = move || {
        let token = cursor.consume_token();
        if token.kind != TokenKind::Eof {Some(token)} else {None}
    };
    let mut res = std::iter::once(first).chain(std::iter::from_fn(consumer));

    if first_is_eof {
        // make sure we return empty iterator instead of one with Eofs
        res.next(); 
        res.next();
    }
    res
}


// pub fn static_unit_table(s: &str) -> Option<TokenKind> {
//     use TokenKind::*;
//     match s {
//         "em" => Some(EMS),
//         "ex" => Some(EXS),
//         "px" | "cm" | "mm" | "in" | "pt" | "pc" => Some(Length),
//         "deg" | "rad" | "grad" => Some(Angle),
//         "ms" | "s" => Some(Time),
//         "hz" | "khz" => Some(Freq),
//         "%" | "fr" => Some(Percentage),
//         "dpi" | "dpcm" => Some(Resolution),
//         "cqw" | "cqh" | "cqi" | "cqb" | "cqmin" | "cqmax" => Some(ContainerQueryLength),
//         _ => None
//     }
// }


impl Cursor<'_> {

    /// Cfr. [CSS Syntax Spec 4.3.1. Consume a token](https://drafts.csswg.org/css-syntax/#consume-token)
    /// Modified to delay bump to avoid reconsumption.
    pub fn consume_token(&mut self) -> Token {
        let unicode_ranges_allowed = true;

        if self.is_eof() {return Token::new(TokenKind::Eof, 0)}

        let first_char = self.first();

        use TokenKind::*;
        let token_kind = match first_char {
            '/' => {
                self.bump();
                match self.first() {
                    '*' => self.consume_comment(),
                    _ => DelimSlash 
                }
            }, 
            c if is_white_space(c) => {
                self.bump();
                self.consume_whitespace()
            },
            '"' => {
                self.bump();
                self.consume_string('"')
            },
            '\'' => {
                self.bump();
                self.consume_string('\'')
            },
            '#' => {
                self.bump();
                if Self::is_ident_mid_char(self.first()) || Self::is_valid_escape(self.first(), self.second()) {
                    if self.is_ident_seq_start() {
                        self.consume_ident_seq();
                        IdHash
                    } else {
                        self.consume_ident_seq();
                        UnrestrictedHash
                    }
                } else {
                    DelimHash
                }
            },
            '+' => {
                if self.is_number_seq_start() {
                    // Reconsume
                    self.consume_numeric_token()
                } else {
                    self.bump();
                    DelimPlus
                }
            },
            '-' =>
                if self.is_number_seq_start() {
                    // RECONSUME
                    self.consume_numeric_token()
                } else if self.second() == '-' && self.third() == '>' {
                    self.bump_three();
                    CDC
                } else if self.is_ident_seq_start() {
                    // RECONSUME
                    self.consume_ident_like()
                } else {
                    self.bump();
                    DelimHyphenMinus
                },
            '.' => if self.is_number_seq_start() {
                    // RECONSUME
                    debug_assert!(self.is_number_seq_start());
                    self.consume_numeric_token()
                } else {
                    self.bump();
                    DelimFullStop
                },
            '<' => if self.second() == '!' && self.third() == '-' && self.fourth() == '-' {
                    self.bump_four();
                    CDO
                } else {
                    self.bump();
                    DelimLessThanSign
                },
            '@' => {
                self.bump();
                if self.is_ident_seq_start() {
                    self.consume_ident_seq();
                    AtKeyword
                } else {
                    DelimCommercialAt
                }
            },
            '\\' => if Self::is_valid_escape_second_char(self.second()) {
                    // RECONSUME
                    self.consume_ident_like()
                } else {
                    self.bump();
                    DelimUnexpectedSolidus
                },
            c if c.is_ascii_digit() => {
                // RECONSUME
                self.consume_numeric_token()
            },
            'U' | 'u' => 
                if unicode_ranges_allowed && self.is_unicode_range_start() {
                    // RECONSUME
                    self.consume_unicode_range()
                } else {
                    // RECONSUME
                    self.consume_ident_like()
                },
            c if Self::is_ident_start_char(c) => {
                    // RECONSUME
                    self.consume_ident_like()
                },
            ',' => {self.bump(); Comma},
            ':' => {self.bump(); Colon},
            ';' => {self.bump(); Semicolon},
            '(' => {self.bump(); OpenParen},
            ')' => {self.bump(); CloseParen},
            '[' => {self.bump(); OpenBracket},
            ']' => {self.bump(); CloseBracket},
            '{' => {self.bump(); OpenCurly},
            '}' => {self.bump(); CloseCurly},
            '>' => {self.bump(); DelimGreaterThanSign},
            '!' => {self.bump(); DelimExclamation},
            EOF_CHAR => {self.bump(); Eof},
            _ => {self.bump(); Unknown}
        };
        let res = Token::new(token_kind, self.pos_within_token());
        self.reset_pos_within_token();
        res
    }

    /// https://drafts.csswg.org/css-syntax/#consume-comments
    /// PRECONDITION: consumed '/', next char is '*'
    /// Contrary to spec, we return a token, and do not check for another immediately succeeding comment
    fn consume_comment(&mut self) -> TokenKind {
        let mut success = false;
        let mut hot = false;
        self.bump_while_first(|ch| {
            if hot && ch == '/' {
                success = true;
                return false;
            }
            hot = ch == '*';
            return true;
        });
        debug_assert!(!success || self.first() == '/');
        if success {self.bump();} // consume trailing '/'
        TokenKind::Comment
    }

    /// https://drafts.csswg.org/css-syntax/#whitespace
    /// Consume as much whitespace as possible
    fn consume_whitespace(&mut self) -> TokenKind {
        self.bump_while_first(|ch| is_white_space(ch));
        TokenKind::WhiteSpace
    }

    /// https://drafts.csswg.org/css-syntax/#consume-a-string-token
    /// Modified to delay bump to avoid reconsumption.
    /// Returns String or BadString
    fn consume_string(&mut self, delimiter: char) -> TokenKind {
        loop{
            match self.first() {
                c if c == delimiter => {
                    self.bump();
                    return TokenKind::String
                },
                EOF_CHAR => {
                    self.bump();
                    return TokenKind::BadString
                },
                c if Self::is_new_line_non_preprocessed(c) => {
                    // NO RECONSUME BECAUSE WE DO NOT BUMP IN THE LOOP
                    return TokenKind::BadString
                },
                '\\' => {
                    self.bump();
                    let first = self.first();
                    if first == EOF_CHAR {

                    } else if Self::is_new_line_non_preprocessed(first) {
                        self.bump();
                    } else {
                        // valid escape
                        self.consume_escaped();
                    }
                },
                _ => {self.bump();}
            }
        }
    }

    /// https://drafts.csswg.org/css-syntax/#consume-url-token
    /// Return Url or BadUrl
    /// PRECONDITION: initial "url(" has just been consumed
    /// PRECONDITION: called to consume an unquoted value (e.g. url(foo) instead of url("foo"))
    ///             a quoted value in "url()" is parsed as Function (see consume_ident_like)
    fn consume_url(&mut self) -> TokenKind {
        self.consume_whitespace();
        while let Some(curr) = self.bump() {
            match curr {
                ')' => return TokenKind::Url,
                EOF_CHAR => todo!("parse error which returns non error token?"),
                c if is_white_space(c) => {
                    self.consume_whitespace();
                    match self.first() {
                        ')' => {
                            self.bump();
                            return TokenKind::Url
                        },
                        EOF_CHAR => {
                            self.bump();
                            todo!("parse error which returns non error token?")
                        },
                        _ => {
                            return self.consume_bad_url_remnants()
                        }
                    }
                },
                c if c == '"' || c == '\'' || c == '(' || non_printable_char(c) => {
                    return self.consume_bad_url_remnants()
                },
                '\\' => {
                    if Self::is_valid_escape_second_char(self.first()) {
                        self.consume_escaped();
                    } else {
                        return self.consume_bad_url_remnants()
                    }
                },
                _ => {}
            }
        }
        todo!("parse error which returns non error token?")
    }


    /// https://drafts.csswg.org/css-syntax/#consume-the-remnants-of-a-bad-url
    /// Returns BadUrl
    fn consume_bad_url_remnants(&mut self) -> TokenKind {
        while let Some(curr) = self.bump() {
            match curr {
                ')' | EOF_CHAR => {return TokenKind::BadUrl},
                c if Self::is_valid_escape(c, self.first()) => {
                    self.consume_escaped();
                },
                _ => {}
            }
        }
        return TokenKind::BadUrl
    }

    /// https://drafts.csswg.org/css-syntax/#consume-numeric-token
    fn consume_numeric_token(&mut self) -> TokenKind {
        self.consume_number();
        if self.is_ident_seq_start() {
            self.consume_ident_seq();
            // TODO maybe flatten dimension units like VSCode CSS language service
            return TokenKind::Dimension
        } else if self.first() == '%' {
            self.bump();
            return TokenKind::Percentage
        }
        return TokenKind::Number
    }

    /// https://drafts.csswg.org/css-syntax/#consume-a-number
    /// PRECONDITION: self.is_number_seq_start() returns true
    /// Unlike the spec, we do not keep any extra information about the number, 
    /// this could be added if necessary.
    fn consume_number(&mut self) {
        debug_assert!(self.is_number_seq_start());
        let first: char = self.bump().unwrap_or('\0');
        if first == '+' || first == '-' {
            self.bump();
        }
        self.bump_while_first(|ch| ch.is_ascii_digit());

        if self.first() == '.' && self.second().is_ascii_digit() {
            self.bump();
            self.bump_while_first(|ch| ch.is_ascii_digit());
        }

        let first = self.first();
        if first == 'e' || first == 'E' {
            let mut second = self.second();
            if second == '+' || second == '-' {
                second = self.third();
            }
            if second.is_ascii_digit() {
                self.bump();
                let first = self.first();
                if first == '+' || first == '-' {
                    self.bump();
                }
                self.bump_while_first(|ch| ch.is_ascii_digit());
            }
        }
    }

    /// https://drafts.csswg.org/css-syntax/#consume-name
    /// Other name for ident sequence
    // #[inline]
    // fn consume_name(&mut self) {
    //     self.consume_ident_seq()
    // }

    /// https://drafts.csswg.org/css-syntax/#consume-name
    /// consumes the largest name that can be formed from adjacent code points in the stream, starting from the first
    /// Modified to delay bump to avoid reconsumption.
    /// PRECONDITION: self.is_ident_seq_start() returns true
    fn consume_ident_seq(&mut self) {
        loop {
            let curr = self.first();
            if curr == EOF_CHAR {
                break
            }

            if Self::is_ident_mid_char(curr) {
                self.bump();
            } else if Self::is_valid_escape(curr, self.second()) {
                self.bump();
                self.consume_escaped();
            } else {
                // NO RECONSUME BECAUSE WE DO NOT BUMP IN THE LOOP
                break
            }
        }
    }

    /// https://drafts.csswg.org/css-syntax/#consume-ident-like-token
    /// Returns Ident, Function, Url, or BadUrl
    fn consume_ident_like(&mut self) -> TokenKind {
        let starts_with_url = match self.first() {
            'u' | 'U' => match self.second() {
                'r' | 'R' => match self.third() {
                    'l' | 'L' => true,
                    _ => false,
                },
                _ => false,
            },
            _ => false,
        };
        self.consume_ident_seq();
        if starts_with_url && self.first() == '(' {
            self.bump();
            while is_white_space(self.first()) && is_white_space(self.second()) {
                self.bump();
            }
            let first = self.first();
            if first == '\'' || first == '"' || 
                (is_white_space(first) && (match self.second() {'\'' | '"' => true, _ => false})) 
            {
                return TokenKind::Function
            } else {
                return self.consume_url()
            }
        } else if self.first() == '(' {
            self.bump();
            return TokenKind::Function
        } else {
            return TokenKind::Ident
        }

    }

    /// https://drafts.csswg.org/css-syntax/#consume-an-escaped-code-point
    /// PRECONDITION: '\\' has just been consumed
    /// PRECONDITION: Self::is_valid_escape('\\', self.first()) returns true
    fn consume_escaped(&mut self) {
        let curr = self.bump().unwrap_or(EOF_CHAR);
        match curr {
            c if c.is_ascii_hexdigit() => {
                let mut hex_str = String::new();
                hex_str.push(curr);
                self.bump_while_first(|ch| {
                    if ch.is_ascii_hexdigit() && hex_str.len() < 6 {
                        hex_str.push(ch);
                        return true
                    }
                    false
                });
                println!("f {} s{} t{}", self.first(), self.second(), self.third());
                if is_white_space(self.first()) {self.bump();} 
                let hex_val = hex_string_to_num(&hex_str).unwrap();
                if hex_val == 0 || is_surrogate_unicode_code_point(hex_val) || exceeds_max_unicode_code_point(hex_val) {
                    todo!("return replacement char?")
                }
            },
            EOF_CHAR => todo!("return replacement char? (spec says it's parse error)"),
            _ => {}
        }
    }

    /// https://drafts.csswg.org/css-syntax/#consume-unicode-range-token
    /// Returns UnicodeRange
    /// PRECONDITION: self.is_unicode_range_start() returns true]
    /// Note: This token is not produces under normal circumstances,
    /// but is only called during [consume the value of a unicode-range descriptor](https://drafts.csswg.org/css-syntax/#consume-the-value-of-a-unicode-range-descriptor),
    /// which itself is only called as a special case for parsing the [unicode range descriptor](https://drafts.csswg.org/css-fonts-4/#descdef-font-face-unicode-range)
    /// this single invocation in the entire language is due to a bad syntax design in early CSS.
    fn consume_unicode_range(&mut self) -> TokenKind {
        self.bump_two(); // U+
        let mut count_hex_question = 0;
        self.bump_while_first(|ch| {
            if ch.is_ascii_hexdigit() && count_hex_question < 6 {
                count_hex_question += 1;
                return true
            }
            false
        });
        
        if count_hex_question < 6 {
            let mut encountered_question = false;
            self.bump_while_first(|ch| {
                if ch == '?' {
                    encountered_question = true;
                    count_hex_question += 1;
                    return count_hex_question <= 6
                }
                return false
            });
        }
        if self.first() == '-' && self.second().is_ascii_hexdigit() {
            self.bump();
            let mut count_hex_question = 0;
            self.bump_while_first(|ch| {
                if ch.is_ascii_hexdigit() && count_hex_question < 6 {
                    count_hex_question += 1;
                    return true
                }
                false
            });
        }
        return TokenKind::UnicodeRange
    }

    #[inline]
    /// https://drafts.csswg.org/css-syntax/#starts-a-unicode-range
    fn is_unicode_range_start(&self) -> bool {
        debug_assert!(matches!(self.first(), 'u' | 'U'));
        self.second() == '+' && match self.third() {'?' => true, c if c.is_ascii_hexdigit() => true, _ => false} 
    }


    #[inline]
    /// https://drafts.csswg.org/css-syntax/#starts-with-a-number
    fn is_number_seq_start(&self) -> bool {
        match self.first() {
            '+' | '-' => match self.second() {
                c if c.is_ascii_digit() => true,
                '.' => match self.third() {
                    c if c.is_ascii_digit() => true,
                    _ => false
                },
                _ => false
            },
            '.' => self.second().is_ascii_digit(),
            c if c.is_ascii_digit() => true,
            _ => false
        }
    }
    

    #[inline]
    /// https://drafts.csswg.org/css-syntax/#would-start-an-identifier
    fn is_ident_seq_start(&self) -> bool {
        match self.first() {
            '-' => {
                match self.second() {
                    c if c == '-' || Self::is_ident_start_char(c)  => { 
                        true 
                    },
                    '\\' => Self::is_valid_escape_second_char(self.third()),
                    _ => false,
                }
            },
            c if Self::is_ident_start_char(c) => true,
            '\\' => Self::is_valid_escape_second_char(self.second()),
            _ => false
        }
    }

    /// https://drafts.csswg.org/css-syntax/#ident-start-code-point
    #[inline]
    fn is_ident_start_char(c: char) -> bool {
        c.is_ascii_alphabetic() || !c.is_ascii() || c == '_'
    }

    /// https://drafts.csswg.org/css-syntax/#ident-code-point
    #[inline]
    fn is_ident_mid_char(c: char) -> bool {
        Self::is_ident_start_char(c) || c.is_ascii_digit() || c == '-'
    }

    /// https://drafts.csswg.org/css-syntax/#check-if-two-code-points-are-a-valid-escape
    #[inline]
    fn is_valid_escape(first_char: char, second_char: char) -> bool {
        if first_char != '\\' {
            return false
        }
        !Self::is_new_line_non_preprocessed(second_char)
    }

    /// https://drafts.csswg.org/css-syntax/#check-if-two-code-points-are-a-valid-escape
    #[inline]
    fn is_valid_escape_second_char(ch: char) -> bool {
        !Self::is_new_line_non_preprocessed(ch)
    }

    #[inline]
    fn is_new_line_non_preprocessed(first_char: char) -> bool {
        first_char == '\n' || first_char == '\r' || first_char == '\x0C'
    }
}

fn is_white_space(ch: char) -> bool {
    ch == ' ' || ch == '\t' || ch == '\n' || ch == '\x0c' || ch == '\r'
}

/// https://infra.spec.whatwg.org/#surrogate
fn is_surrogate_unicode_code_point(u: u32) -> bool {
    is_leading_surrogate_unicode_code_point(u) || is_trailing_surrogate_unicode_code_point(u)
}

/// https://infra.spec.whatwg.org/#leading-surrogate
fn is_leading_surrogate_unicode_code_point(u: u32) -> bool {
    hex_string_to_num("D800").unwrap() <= u && u <= hex_string_to_num("DBFF").unwrap()
}

/// https://infra.spec.whatwg.org/#trailing-surrogate
fn is_trailing_surrogate_unicode_code_point(u: u32) -> bool {
    hex_string_to_num("DC00").unwrap() <= u && u <= hex_string_to_num("DFFF").unwrap()
}

/// https://drafts.csswg.org/css-syntax/#maximum-allowed-code-point
fn exceeds_max_unicode_code_point(u: u32) -> bool {
    hex_string_to_num("10FFFF").unwrap() < u
}

/// https://drafts.csswg.org/css-syntax/#non-printable-code-point
fn non_printable_char(c: char) -> bool {
    let u = c as u32;
    (/* 0 <= u && */ u <= hex_string_to_num("0008").unwrap()) || 
        u == hex_string_to_num("000B").unwrap() ||
        (hex_string_to_num("000E").unwrap() <= u && u <= hex_string_to_num("001F").unwrap()) ||
        u == hex_string_to_num("007F").unwrap()
}

#[allow(clippy::identity_op)]
#[allow(clippy::erasing_op)]
fn hex_string_to_num(s: &str) -> Result<u32, ()> {
    let mut res = 0;
    for (e, c) in s.chars().rev().enumerate() {
        res += c.to_digit(16).ok_or(())? * 16_u32.pow(e.try_into().map_err(|_| ())?);
    }
    return Ok(res)
}