#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    _INVALID,
    Ident,
    AtKeyword,
    String,
    BadString,
    UnquotedString,
    Hash,
    Num,
    Percentage,
    Dimension,
    UnicodeRange,
    CDO, // <!--
    CDC, // -->
    Colon,
    SemiColon,
    CurlyL,
    CurlyR,
    ParenthesisL,
    ParenthesisR,
    BracketL,
    BracketR,
    Whitespace,
    Includes,
    Dashmatch,         // |=
    SubstringOperator, // *=
    PrefixOperator,    // ^=
    SuffixOperator,    // $=
    Delim,
    EMS, // 3em
    EXS, // 3ex
    Length,
    Angle,
    Time,
    Freq,
    Exclamation,
    Resolution,
    Comma,
    Charset,
    EscapedJavaScript,
    BadEscapedJavaScript,
    Comment,
    SingleLineComment,
    EOF,
    ContainerQueryLength,
    CustomToken,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub text: String,
    pub offset: usize,
    pub length: usize,
}

/// `length` and `position` are byte indices into the string
pub struct MultiLineStream {
    pub source: String,
    pub length: usize,
    pub position: usize,
}

impl MultiLineStream {
    pub fn new(source: String) -> Self {
        return MultiLineStream {
            length: source.len(),
            source,
            position: 0,
        };
    }

    // `from` is byte index at valid UTF-8 code point
    pub fn substring_to_curr(&self, from: usize) -> &str {
        return &self.source[from..self.position];
    }

    /// `from` and `to` are byte indices and must be located at UTF-8 code point boundary
    pub fn substring(&self, from: usize, to: usize) -> &str {
        return &self.source[from..to];
    }

    pub fn eos(&self) -> bool {
        return self.length <= self.position;
    }

    pub fn char_at(&self, byte_offset: usize) -> char {
        return self.source[byte_offset..].chars().next().unwrap();
    }

    pub fn char_at_infallible(&self, byte_offset: usize) -> char {
        return self.source[byte_offset..].chars().next().unwrap_or('\0');
    }

    pub fn get_position(&self) -> usize {
        return self.position;
    }

    pub fn set_position(&mut self, position: usize) {
        self.position = position;
    }

    pub fn set_source(&mut self, source: String) {
        self.length = source.len();
        self.position = 0;
        self.source = source;
    }

    pub fn take_source(&mut self) -> String {
        return std::mem::take(&mut self.source);
    }

    /// `step` is byte count
    pub fn advance(&mut self, byte_step: usize) {
        self.position += byte_step;
    }

    // Caller ensure current position is at valid UTF8 code point and before EOF
    pub fn curr_char(&self) -> char {
        return self.char_at(self.position);
    }

    // Returns '\0' if current position at or after EOF
    pub fn curr_char_infallible(&self) -> char {
        return self.char_at_infallible(self.position);
    }

    pub fn next_char(&mut self) -> char {
        let ch = self.source.chars().nth(self.position).unwrap_or('\0');
        self.position += ch.len_utf8();
        return ch;
    }

    pub fn peek_char(&mut self, n: usize) -> Option<char> {
        return self.source[self.position..].chars().nth(n);
    }

    pub fn peek_char_infallible(&mut self, n: usize) -> char {
        return self.peek_char(n).unwrap_or('\0');
    }

    pub fn lookback_char(&mut self, n: usize) -> Option<char> {
        return self.source[..self.position].chars().rev().nth(n);
    }

    /// Caller ensures this is only called when not at EOF
    pub fn advance_if_char(&mut self, c: char) -> bool {
        if c == self.curr_char_infallible() {
            self.position += c.len_utf8();
            return true;
        }
        return false;
    }

    pub fn advance_if_chars(&mut self, chars: &str) -> bool {
        if self.source[self.position..].starts_with(chars) {
            self.position += chars.len();
            return true;
        }
        return false;
    }

    pub fn advance_while_char<F>(&mut self, mut condition: F) -> usize
    where
        F: FnMut(char) -> bool,
    {
        if self.position >= self.source.len() {
            return 0;
        }
        let start_pos = self.position;
        let mut c;
        while self.position < self.length {
            c = self.char_at(self.position);
            if !condition(c) {
                break;
            }
            self.position += c.len_utf8();
        }
        return self.position - start_pos;
    }
}

const _TLD: char = '~';
const _HAT: char = '^';
const _EQS: char = '=';
const _PIP: char = '|';
const _MIN: char = '-';
const _USC: char = '_';
const _PRC: char = '%';
const _MUL: char = '*';
const _LPA: char = '(';
const _RPA: char = ')';
const _LAN: char = '<';
const _RAN: char = '>';
const _ATS: char = '@';
const _HSH: char = '#';
const _DLR: char = '$';
const _BSL: char = '\\';
const _FSL: char = '/';
const _NWL: char = '\n';
const _CAR: char = '\r';
const _LFD: char = '\x0c';
const _DQO: char = '"';
const _SQO: char = '\'';
const _WSP: char = ' ';
const _TAB: char = '\t';
const _SEM: char = ';';
const _COL: char = ':';
const _CUL: char = '{';
const _CUR: char = '}';
const _BRL: char = '[';
const _BRR: char = ']';
const _CMA: char = ',';
const _DOT: char = '.';
const _BNG: char = '!';
const _QSM: char = '?';
const _PLS: char = '+';

pub fn static_token(c: char) -> Option<TokenType> {
    match if c == _SEM {
        TokenType::SemiColon
    } else if c == _COL {
        TokenType::Colon
    } else if c == _CUL {
        TokenType::CurlyL
    } else if c == _CUR {
        TokenType::CurlyR
    } else if c == _BRL {
        TokenType::BracketL
    } else if c == _BRR {
        TokenType::BracketR
    } else if c == _LPA {
        TokenType::ParenthesisL
    } else if c == _RPA {
        TokenType::ParenthesisR
    } else if c == _CMA {
        TokenType::Comma
    } else {
        TokenType::_INVALID
    } {
        TokenType::_INVALID => return None,
        a => return Some(a),
    }
}

pub fn static_unit_table(s: &str) -> Option<TokenType> {
    match if s == "em" {
        TokenType::EMS
    } else if s == "ex" {
        TokenType::EXS
    } else if s == "px" || s == "cm" || s == "mm" || s == "in" || s == "pt" || s == "pc" {
        TokenType::Length
    } else if s == "deg" || s == "rad" || s == "grad" {
        TokenType::Angle
    } else if s == "ms" || s == "s" {
        TokenType::Time
    } else if s == "hz" || s == "khz" {
        TokenType::Freq
    } else if s == "%" || s == "fr" {
        TokenType::Percentage
    } else if s == "dpi" || s == "dpcm" {
        TokenType::Resolution
    } else if s == "cqw" || s == "cqh" || s == "cqi" || s == "cqb" || s == "cqmin" || s == "cqmax" {
        TokenType::ContainerQueryLength
    } else {
        TokenType::_INVALID
    } {
        TokenType::_INVALID => return None,
        a => return Some(a),
    }
}

pub struct Scanner {
    pub stream: MultiLineStream,
    pub ignore_comments: bool,
    pub ignore_whitespace: bool,
    pub in_url: bool,
}

impl Scanner {
    pub fn new(
        source: String,
        ignore_comments: bool,
        ignore_whitespace: bool,
        in_url: bool,
    ) -> Self {
        return Scanner {
            stream: MultiLineStream::new(source),
            ignore_comments,
            ignore_whitespace,
            in_url,
        };
    }

    pub fn finish_token(&self, offset: usize, token_type: TokenType, text: String) -> Token {
        return Token {
            offset,
            length: self.stream.position - offset,
            token_type,
            text,
        };
    }

    pub fn finish_token_auto_text(&self, offset: usize, token_type: TokenType) -> Token {
        return Token {
            offset,
            length: self.stream.position - offset,
            token_type,
            text: self.stream.substring_to_curr(offset).to_string(),
        };
    }

    pub fn substring(&self, offset: usize, length: usize) -> &str {
        return self.stream.substring(offset, offset + length);
    }

    pub fn get_position(&self) -> usize {
        return self.stream.get_position();
    }

    pub fn set_position(&mut self, position: usize) {
        self.stream.set_position(position);
    }

    pub fn set_source(&mut self, input: String) {
        self.stream = MultiLineStream::new(input);
    }

    pub fn scan_unquoted_string(&mut self) -> Option<Token> {
        let offset = self.stream.get_position();
        let mut content: Vec<String> = Vec::new();
        if self._unquoted_string(&mut content) {
            return Some(self.finish_token(offset, TokenType::UnquotedString, content.join("")));
        }
        return None;
    }

    pub fn scan(&mut self) -> Token {
        // processes all whitespace and comments
        let trivia_token = self.trivia();
        if let Some(t) = trivia_token {
            return t;
        }
        let offset = self.stream.get_position();

        // end of file/input
        if self.stream.eos() {
            return self.finish_token_auto_text(offset, TokenType::EOF);
        }
        return self.scan_next(offset);
    }

    pub fn try_scan_unicode(&mut self) -> Option<Token> {
        let offset = self.stream.get_position();
        if !self.stream.eos() && self._unicode_range() {
            return Some(self.finish_token_auto_text(offset, TokenType::UnicodeRange));
        }
        self.stream.set_position(offset);
        return None;
    }

    fn scan_next(&mut self, offset: usize) -> Token {
        if self.stream.advance_if_chars("<!--") {
            return self.finish_token_auto_text(offset, TokenType::CDO);
        }
        if self.stream.advance_if_chars("-->") {
            return self.finish_token_auto_text(offset, TokenType::CDC);
        }

        let mut content: Vec<String> = Vec::new();

        if self.ident(&mut content) {
            return self.finish_token(offset, TokenType::Ident, content.join(""));
        }

        if self.stream.advance_if_char('@') {
            content = vec!['@'.to_string()];
            if self._name(&mut content) {
                let keyword_text = content.join("");
                if keyword_text == "@charset" {
                    return self.finish_token(offset, TokenType::Charset, keyword_text);
                }
                return self.finish_token(offset, TokenType::AtKeyword, keyword_text);
            } else {
                return self.finish_token_auto_text(offset, TokenType::Delim);
            }
        }

        if self.stream.advance_if_char('#') {
            content = vec!['#'.to_string()];
            if self._name(&mut content) {
                return self.finish_token(offset, TokenType::Hash, content.join(""));
            }
            return self.finish_token_auto_text(offset, TokenType::Delim);
        }

        if self.stream.advance_if_char('!') {
            return self.finish_token_auto_text(offset, TokenType::Exclamation);
        }

        if self._number() {
            let pos = self.stream.get_position();
            content = self
                .stream
                .substring(offset, pos)
                .chars()
                .map(|c| c.to_string())
                .collect();
            if self.stream.advance_if_char('%') {
                return self.finish_token_auto_text(offset, TokenType::Percentage);
            }
            if self.ident(&mut content) {
                let dim = self.stream.substring_to_curr(pos).to_lowercase();
                let token_type = static_unit_table(&dim);
                if let Some(tt) = token_type {
                    // known dimension 42px
                    return self.finish_token(offset, tt, content.join(""));
                }
                // unknown dimension 42ft
                return self.finish_token(offset, TokenType::Dimension, content.join(""));
            }
            return self.finish_token_auto_text(offset, TokenType::Num);
        }

        // String, BadString
        content = Vec::new(); // TODO switch to string
        let mut token_type = self._string(&mut content);
        if let Some(tt) = token_type {
            return self.finish_token(offset, tt, content.join(""));
        }

        // single character tokens
        token_type = static_token(self.stream.peek_char_infallible(0));
        if let Some(tt) = token_type {
            let advance_bytes = self.stream.peek_char_infallible(0).len_utf8();
            self.stream.advance(advance_bytes);
            return self.finish_token_auto_text(offset, tt);
        }

        if self.stream.peek_char_infallible(0) == '~' && self.stream.peek_char_infallible(1) == '='
        {
            self.stream.advance("~=".len());
            return self.finish_token_auto_text(offset, TokenType::Includes);
        }

        if self.stream.peek_char_infallible(0) == '|' && self.stream.peek_char_infallible(1) == '='
        {
            self.stream.advance("|=".len());
            return self.finish_token_auto_text(offset, TokenType::Dashmatch);
        }

        if self.stream.peek_char_infallible(0) == '*' && self.stream.peek_char_infallible(1) == '='
        {
            self.stream.advance("*=".len());
            return self.finish_token_auto_text(offset, TokenType::SubstringOperator);
        }

        if self.stream.peek_char_infallible(0) == '^' && self.stream.peek_char_infallible(1) == '='
        {
            self.stream.advance("^=".len());
            return self.finish_token_auto_text(offset, TokenType::PrefixOperator);
        }

        if self.stream.peek_char_infallible(0) == '$' && self.stream.peek_char_infallible(1) == '='
        {
            self.stream.advance("$=".len());
            return self.finish_token_auto_text(offset, TokenType::SuffixOperator);
        }

        self.stream.next_char();
        return self.finish_token_auto_text(offset, TokenType::Delim);
    }

    fn trivia(&mut self) -> Option<Token> {
        let mut offset;
        loop {
            offset = self.stream.get_position();
            if self._whitespace() {
                if !self.ignore_whitespace {
                    return Some(self.finish_token_auto_text(offset, TokenType::Whitespace));
                }
            } else if self.comment() {
                if !self.ignore_comments {
                    return Some(self.finish_token_auto_text(offset, TokenType::Comment));
                }
            } else {
                return None;
            }
        }
    }

    fn comment(&mut self) -> bool {
        if self.stream.advance_if_chars("/*") {
            let mut success = false;
            let mut hot = false;
            self.stream.advance_while_char(|ch| {
                if hot && ch == '/' {
                    success = true;
                    return false;
                }
                hot = ch == '*';
                return true;
            });
            if success {
                self.stream.advance('/'.len_utf8());
            }
            return true;
        }
        return false;
    }

    fn _number(&mut self) -> bool {
        let npeek_char;
        let mut npeek = 0;
        if self.stream.peek_char_infallible(0) == '.' {
            npeek = 1;
            npeek_char = Some('.');
        } else {
            npeek_char = None;
        }
        let ch = self.stream.peek_char_infallible(npeek);
        if ch.is_ascii_digit() {
            self.stream
                .advance(npeek_char.map(|c| c.len_utf8()).unwrap_or(0) + ch.len_utf8());
            self.stream
                .advance_while_char(|ch| return ch.is_ascii_digit() || (npeek == 0 && ch == '.'));
            return true;
        }
        return false;
    }

    fn _newline(&mut self, result: &mut Vec<String>) -> bool {
        let ch = self.stream.peek_char_infallible(0);
        if ch == '\r' || ch == '\x0c' || ch == '\n' {
            self.stream.advance(ch.len_utf8());
            result.push(ch.to_string());
            if ch == '\r' && self.stream.advance_if_char('\n') {
                result.push('\n'.to_string());
            }
            return true;
        }
        return false;
    }

    fn _escape(&mut self, result: &mut Vec<String>, include_new_lines: bool) -> bool {
        let mut ch = self.stream.peek_char_infallible(0);
        if ch == '\\' {
            self.stream.advance('\\'.len_utf8());
            ch = self.stream.peek_char_infallible(0);
            let mut hex_num_count = 0;
            while hex_num_count < 6 && ch.is_ascii_hexdigit() {
                self.stream.advance(ch.len_utf8());
                ch = self.stream.peek_char_infallible(0);
                hex_num_count += 1;
            }
            if hex_num_count > 0 {
                if let Some(h) = hex_string_to_num(
                    self.stream
                        .substring_to_curr(self.stream.get_position() - hex_num_count), // all hex digits are 1 byte so this works
                ) {
                    let mut c = char::decode_utf16(std::iter::once(h as u16));
                    if let Ok(c) = c.nth(0).unwrap() {
                        result.push(c.to_string());
                    }
                }
                // optional whitespace or new line, not part of result text
                if ch == ' ' || ch == '\t' {
                    self.stream.advance(ch.len_utf8());
                } else {
                    self._newline(&mut Vec::new());
                }
                return true;
            }
            if ch != '\r' && ch != '\x0c' && ch != '\n' {
                self.stream.advance(ch.len_utf8());
                result.push(ch.to_string());
                return true;
            } else if include_new_lines {
                return self._newline(result);
            }
        }
        return false;
    }

    fn _string_char(&mut self, close_quote: char, result: &mut Vec<String>) -> bool {
        // not closeQuote, not backslash, not newline
        let ch = self.stream.peek_char_infallible(0);
        if ch != '\0' && ch != close_quote && ch != '\\' && ch != '\r' && ch != '\x0c' && ch != '\n'
        {
            self.stream.advance(ch.len_utf8());
            result.push(ch.to_string());
            return true;
        }
        return false;
    }

    fn _string(&mut self, result: &mut Vec<String>) -> Option<TokenType> {
        if self.stream.peek_char_infallible(0) == '\'' || self.stream.peek_char_infallible(0) == '"'
        {
            let close_quote = self.stream.next_char();
            result.push(close_quote.to_string());

            while self._string_char(close_quote, result) || self._escape(result, true) {
                // loop
            }

            if self.stream.peek_char_infallible(0) == close_quote {
                self.stream.next_char();
                result.push(close_quote.to_string());
                return Some(TokenType::String);
            }
            return Some(TokenType::BadString);
        }
        return None;
    }

    fn _unquoted_char(&mut self, result: &mut Vec<String>) -> bool {
        // not closeQuote, not backslash, not whitespace, not newline
        if let Some(ch) = self.stream.peek_char(0) {
            if ch != '\0'
                && ch != '\\'
                && ch != '\''
                && ch != '"'
                && ch != '('
                && ch != ')'
                && ch != ' '
                && ch != '\t'
                && ch != '\n'
                && ch != '\x0c'
                && ch != '\r'
            {
                self.stream.advance(ch.len_utf8());
                result.push(ch.to_string());
                return true;
            }
        }
        return false;
    }

    fn _unquoted_string(&mut self, result: &mut Vec<String>) -> bool {
        let mut has_content = false;
        while self._unquoted_char(result) || self._escape(result, false) {
            has_content = true;
        }
        return has_content;
    }

    fn _whitespace(&mut self) -> bool {
        let n = self.stream.advance_while_char(|ch| {
            return ch == ' ' || ch == '\t' || ch == '\n' || ch == '\x0c' || ch == '\r';
        });
        return n > 0;
    }

    fn _name(&mut self, result: &mut Vec<String>) -> bool {
        let mut matched = false;
        while self._ident_char(result) || self._escape(result, false) {
            matched = true;
        }
        return matched;
    }

    fn ident(&mut self, result: &mut Vec<String>) -> bool {
        let pos = self.stream.get_position();
        let has_minus = self._minus(result);
        if has_minus {
            if self._minus(result) || self._ident_first_char(result) || self._escape(result, false)
            {
                while self._ident_char(result) || self._escape(result, false) {
                    // loop
                }
                return true;
            }
        } else if self._ident_first_char(result) || self._escape(result, false) {
            while self._ident_char(result) || self._escape(result, false) {
                // loop
            }
            return true;
        }
        self.stream.set_position(pos);
        return false;
    }

    fn _ident_first_char(&mut self, result: &mut Vec<String>) -> bool {
        let ch = self.stream.peek_char_infallible(0);
        if ch == '_' || ch.is_ascii_alphabetic() || !ch.is_ascii() {
            self.stream.advance(ch.len_utf8());
            result.push(ch.to_string());
            return true;
        }
        return false;
    }

    fn _minus(&mut self, result: &mut Vec<String>) -> bool {
        let ch = self.stream.peek_char_infallible(0);
        if ch == '-' {
            self.stream.advance(ch.len_utf8());
            result.push(ch.to_string());
            return true;
        }
        return false;
    }

    fn _ident_char(&mut self, result: &mut Vec<String>) -> bool {
        let ch = self.stream.peek_char_infallible(0);
        if ch == '_' || ch == '-' || ch.is_ascii_alphanumeric() || !ch.is_ascii() {
            self.stream.advance(ch.len_utf8());
            result.push(ch.to_string());
            return true;
        }
        return false;
    }

    fn _unicode_range(&mut self) -> bool {
        // follow https://www.w3.org/TR/CSS21/syndata.html#tokenization and https://www.w3.org/TR/css-syntax-3/#urange-syntax
        // assume u has already been parsed
        if self.stream.advance_if_char('+') {
            let code_points = self.stream.advance_while_char(|ch| ch.is_ascii_hexdigit())
                + self.stream.advance_while_char(|ch| ch == '?');
            if (1..=6).contains(&code_points) {
                if self.stream.advance_if_char('-') {
                    let digits = self.stream.advance_while_char(|ch| ch.is_ascii_hexdigit());
                    if (0..=6).contains(&digits) {
                        return true;
                    }
                } else {
                    return true;
                }
            }
        }
        return false;
    }
}

impl Default for Scanner {
    fn default() -> Self {
        return Scanner::new(String::new(), true, true, false);
    }
}

#[allow(clippy::identity_op)]
#[allow(clippy::erasing_op)]
fn hex_string_to_num(s: &str) -> Option<usize> {
    // `hex_digit_lsb_rev_idx` is the reverse index of the least significant bit of the hex digit
    //      e.g. hex_digit_lsb_rev_idx of `a` in `#04af44` is 12
    fn hex_digit_to_val_by_lsb_rev_idx(c: char, hex_digit_lsb_rev_idx: u32) -> usize {
        let e = hex_digit_lsb_rev_idx;
        let two: usize = 2;
        if c == '0' {
            return 0 * two.pow(e + 3)
                + 0 * two.pow(e + 2)
                + 0 * two.pow(e + 1)
                + 0 * two.pow(e + 0);
        } else if c == '1' {
            return 0 * two.pow(e + 3)
                + 0 * two.pow(e + 2)
                + 0 * two.pow(e + 1)
                + 1 * two.pow(e + 0);
        } else if c == '2' {
            return 0 * two.pow(e + 3)
                + 0 * two.pow(e + 2)
                + 1 * two.pow(e + 1)
                + 0 * two.pow(e + 0);
        } else if c == '3' {
            return 0 * two.pow(e + 3)
                + 0 * two.pow(e + 2)
                + 1 * two.pow(e + 1)
                + 1 * two.pow(e + 0);
        } else if c == '4' {
            return 0 * two.pow(e + 3)
                + 1 * two.pow(e + 2)
                + 0 * two.pow(e + 1)
                + 0 * two.pow(e + 0);
        } else if c == '5' {
            return 0 * two.pow(e + 3)
                + 1 * two.pow(e + 2)
                + 0 * two.pow(e + 1)
                + 1 * two.pow(e + 0);
        } else if c == '6' {
            return 0 * two.pow(e + 3)
                + 1 * two.pow(e + 2)
                + 1 * two.pow(e + 1)
                + 0 * two.pow(e + 0);
        } else if c == '7' {
            return 0 * two.pow(e + 3)
                + 1 * two.pow(e + 2)
                + 1 * two.pow(e + 1)
                + 1 * two.pow(e + 0);
        } else if c == '8' {
            return 1 * two.pow(e + 3)
                + 0 * two.pow(e + 2)
                + 0 * two.pow(e + 1)
                + 0 * two.pow(e + 0);
        } else if c == '9' {
            return 1 * two.pow(e + 3)
                + 0 * two.pow(e + 2)
                + 0 * two.pow(e + 1)
                + 1 * two.pow(e + 0);
        } else if c == 'a' {
            return 1 * two.pow(e + 3)
                + 0 * two.pow(e + 2)
                + 1 * two.pow(e + 1)
                + 0 * two.pow(e + 0);
        } else if c == 'b' {
            return 1 * two.pow(e + 3)
                + 0 * two.pow(e + 2)
                + 1 * two.pow(e + 1)
                + 1 * two.pow(e + 0);
        } else if c == 'c' {
            return 1 * two.pow(e + 3)
                + 1 * two.pow(e + 2)
                + 0 * two.pow(e + 1)
                + 0 * two.pow(e + 0);
        } else if c == 'd' {
            return 1 * two.pow(e + 3)
                + 1 * two.pow(e + 2)
                + 0 * two.pow(e + 1)
                + 1 * two.pow(e + 0);
        } else if c == 'e' {
            return 1 * two.pow(e + 3)
                + 1 * two.pow(e + 2)
                + 1 * two.pow(e + 1)
                + 0 * two.pow(e + 0);
        } else if c == 'f' {
            return 1 * two.pow(e + 3)
                + 1 * two.pow(e + 2)
                + 1 * two.pow(e + 1)
                + 1 * two.pow(e + 0);
        } else {
            panic!()
        }
    }

    // `rev_hex_digit_idx` is index of digit in hex string, in reverse direction
    //      e.g. rev_hex_digit_idx of `a` in `#04af44` is 3
    fn hex_digit_to_val_by_rev_idx(c: char, hex_digit_rev_idx: u32) -> usize {
        return hex_digit_to_val_by_lsb_rev_idx(c, hex_digit_rev_idx * 4);
    }

    if s.chars().any(|c| !c.is_ascii_hexdigit()) {
        return None;
    }
    let s = s.to_lowercase();
    let mut result: usize = 0;
    let mut ch: char;
    let mut i = s.len() - 1;
    let hex_digit_rev_idx = |i: usize| return s.len() - i - 1;
    loop {
        ch = s.chars().nth(i).unwrap();
        result += hex_digit_to_val_by_rev_idx(ch, hex_digit_rev_idx(i) as u32);
        if i == 0 {
            break;
        }
        i -= 1;
    }
    return Some(result);
}

#[cfg(test)]
mod test_css_scanner {
    use super::Scanner;
    use super::TokenType;
    use super::TokenType::*;

    fn ast(
        scanner: &mut Scanner,
        source: &str,
        len: usize,
        offset: usize,
        text: &str,
        token_types: Vec<TokenType>,
    ) {
        scanner.set_source(source.to_owned());
        let token = scanner.scan();
        assert_eq!(token.length, len);
        assert_eq!(token.offset, offset);
        assert_eq!(token.text, text);
        assert_eq!(token.token_type, token_types[0]);
        for tt in &token_types[1..] {
            assert_eq!(scanner.scan().token_type, tt.clone());
        }
        assert_eq!(scanner.scan().token_type, TokenType::EOF);
    }

    #[test]
    fn whitespace() {
        let mut sc = Scanner::default();
        ast(&mut sc, " @", 1, 1, "@", vec![Delim]);
        ast(
            &mut sc,
            " /* comment*/ \n/*comment*/@",
            1,
            26,
            "@",
            vec![Delim],
        );

        sc = Scanner::default();
        sc.ignore_whitespace = false;
        ast(&mut sc, " @", 1, 0, " ", vec![Whitespace, Delim]);
        ast(
            &mut sc,
            "/*comment*/ @",
            1,
            11,
            " ",
            vec![Whitespace, Delim],
        );

        sc = Scanner::default();
        sc.ignore_comments = false;
        ast(
            &mut sc,
            " /*comment*/@",
            11,
            1,
            "/*comment*/",
            vec![Comment, Delim],
        );
        ast(
            &mut sc,
            "/*comment*/ @",
            11,
            0,
            "/*comment*/",
            vec![Comment, Delim],
        );
    }

    #[test]
    fn token_ident() {
        let sc = &mut Scanner::default();
        ast(sc, "\u{060F}rf", 4, 0, "\u{060F}rf", vec![Ident]);
        ast(sc, "über", 5, 0, "über", vec![Ident]);
        ast(sc, "-bo", 3, 0, "-bo", vec![Ident]);
        ast(sc, "_bo", 3, 0, "_bo", vec![Ident]);
        ast(sc, "boo", 3, 0, "boo", vec![Ident]);
        ast(sc, "Boo", 3, 0, "Boo", vec![Ident]);
        ast(sc, "red--", 5, 0, "red--", vec![Ident]);
        ast(sc, "red-->", 5, 0, "red--", vec![Ident, Delim]);
        ast(sc, "--red", 5, 0, "--red", vec![Ident]);
        ast(sc, "--100", 5, 0, "--100", vec![Ident]);
        ast(sc, "---red", 6, 0, "---red", vec![Ident]);
        ast(sc, "---", 3, 0, "---", vec![Ident]);
        ast(sc, "a\\.b", 4, 0, "a.b", vec![Ident]);
        ast(sc, "\\E9motion", 9, 0, "émotion", vec![Ident]);
        ast(sc, "\\E9 dition", 10, 0, "édition", vec![Ident]);
        ast(sc, "\\0000E9dition", 13, 0, "édition", vec![Ident]);
        ast(sc, "S\\0000e9f", 9, 0, "Séf", vec![Ident]);
    }

    #[test]
    fn token_url() {
        let sc = &mut Scanner::default();
        fn assert_url_argument(sc: &mut Scanner, source: &str, text: &str, token_type: TokenType) {
            sc.set_source(source.to_owned());
            let token = sc.scan_unquoted_string();
            assert!(token.is_some());
            let token = token.unwrap();
            assert_eq!(token.length, text.len());
            assert_eq!(token.offset, 0);
            assert_eq!(token.text, text);
            assert_eq!(token.token_type, token_type);
        }

        assert_url_argument(sc, "http://msft.com", "http://msft.com", UnquotedString);
        assert_url_argument(sc, "http://msft.com\"", "http://msft.com", UnquotedString);
    }

    #[test]
    fn token_at_keyword() {
        let sc = &mut Scanner::default();
        ast(sc, "@import", 7, 0, "@import", vec![AtKeyword]);
        ast(sc, "@importttt", 10, 0, "@importttt", vec![AtKeyword]);
        ast(sc, "@imp", 4, 0, "@imp", vec![AtKeyword]);
        ast(sc, "@5", 2, 0, "@5", vec![AtKeyword]);
        ast(sc, "@media", 6, 0, "@media", vec![AtKeyword]);
        ast(sc, "@page", 5, 0, "@page", vec![AtKeyword]);
        ast(sc, "@charset", 8, 0, "@charset", vec![Charset]);
        ast(sc, "@-mport", 7, 0, "@-mport", vec![AtKeyword]);
        ast(
            sc,
            "@\u{00f0}mport",
            8,
            0,
            "@\u{00f0}mport",
            vec![AtKeyword],
        );
        ast(sc, "@apply", 6, 0, "@apply", vec![AtKeyword]);
        ast(sc, "@", 1, 0, "@", vec![Delim]);
    }

    #[test]
    fn token_number() {
        let sc = &mut Scanner::default();
        ast(sc, "1234", 4, 0, "1234", vec![Num]);
        ast(sc, "1.34", 4, 0, "1.34", vec![Num]);
        ast(sc, ".234", 4, 0, ".234", vec![Num]);
        ast(sc, ".234.", 4, 0, ".234", vec![Num, Delim]);
        ast(sc, "..234", 1, 0, ".", vec![Delim, Num]);
    }

    #[test]
    fn token_delim() {
        let sc = &mut Scanner::default();
        ast(sc, "@", 1, 0, "@", vec![Delim]);
        ast(sc, "+", 1, 0, "+", vec![Delim]);
        ast(sc, ">", 1, 0, ">", vec![Delim]);
        ast(sc, "#", 1, 0, "#", vec![Delim]);
        ast(sc, "'", 1, 0, "'", vec![BadString]);
        ast(sc, "\"", 1, 0, "\"", vec![BadString]);
    }

    #[test]
    fn token_hash() {
        let sc = &mut Scanner::default();
        ast(sc, "#import", 7, 0, "#import", vec![Hash]);
        ast(sc, "#-mport", 7, 0, "#-mport", vec![Hash]);
        ast(sc, "#123", 4, 0, "#123", vec![Hash]);
    }

    #[test]
    fn token_dimension_or_percentage() {
        let sc = &mut Scanner::default();
        ast(sc, "3em", 3, 0, "3em", vec![EMS]);
        ast(sc, "4.423ex", 7, 0, "4.423ex", vec![EXS]);
        ast(sc, "3423px", 6, 0, "3423px", vec![Length]);
        ast(sc, "4.423cm", 7, 0, "4.423cm", vec![Length]);
        ast(sc, "4.423mm", 7, 0, "4.423mm", vec![Length]);
        ast(sc, "4.423in", 7, 0, "4.423in", vec![Length]);
        ast(sc, "4.423pt", 7, 0, "4.423pt", vec![Length]);
        ast(sc, "4.423pc", 7, 0, "4.423pc", vec![Length]);
        ast(sc, "4.423deg", 8, 0, "4.423deg", vec![Angle]);
        ast(sc, "4.423rad", 8, 0, "4.423rad", vec![Angle]);
        ast(sc, "4.423grad", 9, 0, "4.423grad", vec![Angle]);
        ast(sc, "4.423ms", 7, 0, "4.423ms", vec![Time]);
        ast(sc, "4.423s", 6, 0, "4.423s", vec![Time]);
        ast(sc, "4.423hz", 7, 0, "4.423hz", vec![Freq]);
        ast(sc, ".423khz", 7, 0, ".423khz", vec![Freq]);
        ast(sc, "3.423%", 6, 0, "3.423%", vec![Percentage]);
        ast(sc, ".423%", 5, 0, ".423%", vec![Percentage]);
        ast(sc, ".423ft", 6, 0, ".423ft", vec![Dimension]);
        ast(sc, "200dpi", 6, 0, "200dpi", vec![Resolution]);
        ast(sc, "123dpcm", 7, 0, "123dpcm", vec![Resolution]);
    }

    #[test]
    fn token_string() {
        let sc = &mut Scanner::default();
        ast(sc, "'farboo'", 8, 0, "'farboo'", vec![String]);
        ast(sc, "\"farboo\"", 8, 0, "\"farboo\"", vec![String]);
        ast(
            sc,
            "\"farbo\u{00f0}\"",
            9,
            0,
            "\"farbo\u{00f0}\"",
            vec![String],
        );
        ast(sc, "\"far\\\"oo\"", 9, 0, "\"far\"oo\"", vec![String]);
        ast(sc, "\"fa\\\noo\"", 8, 0, "\"fa\noo\"", vec![String]);
        ast(sc, "\"fa\\\roo\"", 8, 0, "\"fa\roo\"", vec![String]);
        ast(
            sc,
            "\"fa\\\u{000c}oo\"",
            8,
            0,
            "\"fa\u{000c}oo\"",
            vec![String],
        );
        ast(sc, "'farboo\"", 8, 0, "'farboo\"", vec![BadString]);
        ast(sc, "\"farboo", 7, 0, "\"farboo", vec![BadString]);
        ast(sc, "'", 1, 0, "'", vec![BadString]);
        ast(sc, "\"", 1, 0, "\"", vec![BadString]);
    }

    #[test]
    fn token_cdo() {
        let sc = &mut Scanner::default();
        ast(sc, "<!--", 4, 0, "<!--", vec![CDO]);
        ast(
            sc,
            "<!-\n-",
            1,
            0,
            "<",
            vec![Delim, Exclamation, Delim, Delim],
        );
    }

    #[test]
    fn token_cdc() {
        let sc = &mut Scanner::default();
        ast(sc, "-->", 3, 0, "-->", vec![CDC]);
        ast(sc, "--y>", 3, 0, "--y", vec![Ident, Delim]);
        ast(sc, "--<", 2, 0, "--", vec![Ident, Delim]);
    }

    #[test]
    fn token_misc_delims_and_punct() {
        let sc = &mut Scanner::default();
        ast(sc, ":  ", 1, 0, ":", vec![Colon]);
        ast(sc, ";  ", 1, 0, ";", vec![SemiColon]);
        ast(sc, "{  ", 1, 0, "{", vec![CurlyL]);
        ast(sc, "}  ", 1, 0, "}", vec![CurlyR]);
        ast(sc, "[  ", 1, 0, "[", vec![BracketL]);
        ast(sc, "]  ", 1, 0, "]", vec![BracketR]);
        ast(sc, "(  ", 1, 0, "(", vec![ParenthesisL]);
        ast(sc, ")  ", 1, 0, ")", vec![ParenthesisR]);
    }

    #[test]
    fn token_dashmatch_and_includes() {
        let sc = &mut Scanner::default();
        ast(sc, "~=", 2, 0, "~=", vec![Includes]);
        ast(sc, "~", 1, 0, "~", vec![Delim]);
        ast(sc, "|=", 2, 0, "|=", vec![Dashmatch]);
        ast(sc, "|", 1, 0, "|", vec![Delim]);
        ast(sc, "^=", 2, 0, "^=", vec![PrefixOperator]);
        ast(sc, "$=", 2, 0, "$=", vec![SuffixOperator]);
        ast(sc, "*=", 2, 0, "*=", vec![SubstringOperator]);
    }

    #[test]
    fn comments() {
        let sc = &mut Scanner::default();
        ast(sc, "/*      */", 0, 10, "", vec![EOF]);
        ast(sc, "/*      abcd*/", 0, 14, "", vec![EOF]);
        ast(sc, "/*abcd  */", 0, 10, "", vec![EOF]);
        ast(sc, "/* ab- .-cd  */", 0, 15, "", vec![EOF]);
    }

    #[test]
    fn whitespaces() {
        let sc = &mut Scanner::default();
        ast(sc, " ", 0, 1, "", vec![EOF]);
        ast(sc, "      ", 0, 6, "", vec![EOF]);
    }

    fn assert_token_sequence(scanner: &mut Scanner, source: &str, tokens: Vec<TokenType>) {
        scanner.set_source(source.to_owned());
        let mut token = scanner.scan();
        let mut i = 0;
        while tokens.len() > i {
            assert_eq!(token.token_type, tokens[i]);
            token = scanner.scan();
            i += 1;
        }
    }

    // tests with skipping comments
    #[test]
    fn token_sequence() {
        let sc = &mut Scanner::default();
        assert_token_sequence(sc, "5 5 5 5", vec![Num, Num, Num, Num]);
        assert_token_sequence(sc, "/* 5 4 */-->", vec![CDC]);
        assert_token_sequence(sc, "/* 5 4 */ -->", vec![CDC]);
        assert_token_sequence(sc, "/* \"adaasd\" */ -->", vec![CDC]);
        assert_token_sequence(sc, "/* <!-- */ -->", vec![CDC]);
        assert_token_sequence(sc, "red-->", vec![Ident, Delim]);
        assert_token_sequence(sc, "@ import", vec![Delim, Ident]);
    }
}
