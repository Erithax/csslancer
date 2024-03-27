

#[cfg(test)]
mod test_css_lexer {
    use super::super::{TokenKind, TokenKind::*, tokenize_file};

    fn ast(input: &str, expected_token_kinds: Vec<TokenKind>) {
        //expected_token_kinds.push(TokenKind::Eof);
        let mut tokens = tokenize_file(input);
        println!("tokens: {}", tokenize_file(input).into_iter().map(|t| format!("{:?}", t.kind)).collect::<Vec<std::string::String>>().join(" > "));
        for expected in expected_token_kinds {
            let received = tokens.next();
            assert!(received.is_some(), "less tokens than expected on input `{input}`, expecting token `{expected:?}`");
            let received = received.unwrap().kind;
            assert_eq!(expected, received, "expected token `{expected:?}` but encountered `{received:?}`");
        }
        assert!(tokens.next().is_none());
    }

    #[test]
    fn whitespace() {
        // ast(" ", vec![WhiteSpace]);
        // ast("\t", vec![WhiteSpace]);
        ast(" @", vec![WhiteSpace, DelimCommercialAt]);
        ast(" /* comment*/ \n/*comment*/@", vec![WhiteSpace, Comment, WhiteSpace, Comment, DelimCommercialAt]);
        ast("/*comment*/ @", vec![Comment, WhiteSpace, DelimCommercialAt]);
        ast(" /*comment*/@", vec![WhiteSpace, Comment, DelimCommercialAt]);
        ast("/*comment*/ @", vec![Comment, WhiteSpace, DelimCommercialAt]);
    }

    #[test]
    fn token_ident() {
        ast("\u{060F}rf", vec![Ident]);
        ast("über", vec![Ident]);
        ast("-bo", vec![Ident]);
        ast("_bo", vec![Ident]);
        ast("boo", vec![Ident]);
        ast("Boo", vec![Ident]);
        ast("red--", vec![Ident]);
        ast("red-->", vec![Ident, DelimGreaterThanSign]);
        ast("--red", vec![Ident]);
        ast("--100", vec![Ident]);
        ast("---red", vec![Ident]);
        ast("---", vec![Ident]);
        ast("a\\.b", vec![Ident]);
        ast("\\E9motion", vec![Ident]);
        ast("\\E9 dition", vec![Ident]);
        ast("\\0000E9dition", vec![Ident]);
        ast("S\\0000e9f", vec![Ident]);
    }

    // #[test]
    // fn token_url() {
    //     fn assert_url_argument(source: &str, text: &str, token_type: TokenType) {
    //         sc.set_source(source.to_owned());
    //         let token = sc.scan_unquoted_string();
    //         assert!(token.is_some());
    //         let token = token.unwrap();
    //         assert_eq!(token.length, text.len());
    //         assert_eq!(token.offset, 0);
    //         assert_eq!(token.text, text);
    //         assert_eq!(token.token_type, token_type);
    //     }

    //     assert_url_argument("http://msft.com", "http://msft.com", UnquotedString);
    //     assert_url_argument("http://msft.com\"", "http://msft.com", UnquotedString);
    // }

    #[test]
    fn token_charset() {
        ast("@charset \"utf-8\";", vec![Charset]);
        ast(" @charset \"utf-8\";", vec![WhiteSpace, AtKeyword, WhiteSpace, String, Semicolon]); 
    }

    #[test]
    fn token_at_keyword() {
        ast("@import",vec![AtKeyword]);
        ast("@importttt", vec![AtKeyword]);
        ast("@imp", vec![AtKeyword]);
        //ast("@5", vec![AtKeyword]); TODO: check why VSCode has this
        ast("@media", vec![AtKeyword]);
        ast("@page", vec![AtKeyword]);
        ast("@-mport", vec![AtKeyword]);
        ast(
            "@\u{00f0}mport",
            vec![AtKeyword],
        );
        ast("@apply", vec![AtKeyword]);
        ast("@", vec![DelimCommercialAt]);
    }

    #[test]
    fn token_number() {
        ast("1234", vec![Number]);
        ast("1.34", vec![Number]);
        ast(".234", vec![Number]);
        ast(".234.", vec![Number, DelimFullStop]);
        ast("..234", vec![DelimFullStop, Number]);
    }

    #[test]
    fn token_delim() {
        ast("@", vec![DelimCommercialAt]);
        ast("+", vec![DelimPlus]);
        ast(">", vec![DelimGreaterThanSign]);
        ast("#", vec![DelimHash]);
        ast("'", vec![BadString]);
        ast("\"", vec![BadString]);
    }

    #[test]
    fn token_hash() {
        ast("#import", vec![IdHash]);
        ast("#-mport", vec![IdHash]);
        ast("#123", vec![UnrestrictedHash]);
    }

    // #[test]
    // fn token_dimension_or_percentage() {
    //     ast("3em", vec![EMS]);
    //     ast("4.423ex", vec![EXS]);
    //     ast("3423px", vec![Length]);
    //     ast("4.423cm", vec![Length]);
    //     ast("4.423mm", vec![Length]);
    //     ast("4.423in", vec![Length]);
    //     ast("4.423pt", vec![Length]);
    //     ast("4.423pc", vec![Length]);
    //     ast("4.423deg", vec![Angle]);
    //     ast("4.423rad", vec![Angle]);
    //     ast("4.423grad", vec![Angle]);
    //     ast("4.423ms", vec![Time]);
    //     ast("4.423s", vec![Time]);
    //     ast("4.423hz", vec![Freq]);
    //     ast(".423khz", vec![Freq]);
    //     ast("3.423%", vec![Percentage]);
    //     ast(".423%", vec![Percentage]);
    //     ast(".423ft", vec![Dimension]);
    //     ast("200dpi", vec![Resolution]);
    //     ast("123dpcm", vec![Resolution]);
    // }

    #[test]
    fn token_dimension_or_percentage() {
        ast("3em", vec![Dimension]);
        ast("4.423ex", vec![Dimension]);
        ast("3423px", vec![Dimension]);
        ast("4.423cm", vec![Dimension]);
        ast("4.423mm", vec![Dimension]);
        ast("4.423in", vec![Dimension]);
        ast("4.423pt", vec![Dimension]);
        ast("4.423pc", vec![Dimension]);
        ast("4.423deg", vec![Dimension]);
        ast("4.423rad", vec![Dimension]);
        ast("4.423grad", vec![Dimension]);
        ast("4.423ms", vec![Dimension]);
        ast("4.423s", vec![Dimension]);
        ast("4.423hz", vec![Dimension]);
        ast(".423khz", vec![Dimension]);
        ast("3.423%", vec![Percentage]);
        ast(".423%", vec![Percentage]);
        ast(".423ft", vec![Dimension]);
        ast("200dpi", vec![Dimension]);
        ast("123dpcm", vec![Dimension]);
    }

    #[test]
    fn token_string() {
        ast("'farboo'", vec![String]);
        ast("\"farboo\"", vec![String]);
        ast("\"farbo\u{00f0}\"", vec![String]);
        ast("\"far\\\"oo\"", vec![String]);
        ast("\"fa\\\noo\"", vec![String]);
        ast("\"fa\\\roo\"", vec![String]);
        ast("\"fa\\\u{000c}oo\"", vec![String]);
        ast("'farboo\"", vec![BadString]);
        ast("\"farboo", vec![BadString]);
        ast("'", vec![BadString]);
        ast("\"", vec![BadString]);
    }

    #[test]
    fn token_cdo() {
        ast("<!--", vec![CDO]);
        ast("<!-\n-", vec![DelimLessThanSign, DelimExclamation, DelimHyphenMinus, WhiteSpace, DelimHyphenMinus]);
    }

    #[test]
    fn token_cdc() {
        ast("-->", vec![CDC]);
        ast("--y>", vec![Ident, DelimGreaterThanSign]);
        ast("--<", vec![Ident, DelimLessThanSign]);
    }

    #[test]
    fn token_misc_delims_and_punct() {
        ast(":  ", vec![Colon, WhiteSpace]);
        ast(";  ", vec![Semicolon, WhiteSpace]);
        ast("{  ", vec![OpenCurly, WhiteSpace]);
        ast("}  ", vec![CloseCurly, WhiteSpace]);
        ast("[  ", vec![OpenBracket, WhiteSpace]);
        ast("]  ", vec![CloseBracket, WhiteSpace]);
        ast("(  ", vec![OpenParen, WhiteSpace]);
        ast(")  ", vec![CloseParen, WhiteSpace]);
    }

    // #[test]
    // fn token_dashmatch_and_includes() {
    //     ast("~=", vec![Includes]);
    //     ast("~", vec![Delim]);
    //     ast("|=", vec![Dashmatch]);
    //     ast("|", vec![Delim]);
    //     ast("^=", vec![PrefixOperator]);
    //     ast("$=", vec![SuffixOperator]);
    //     ast("*=", vec![SubstringOperator]);
    // }

    #[test]
    fn comments() {
        ast("/*      */", vec![Comment]);
        ast("/*      abcd*/", vec![Comment]);
        ast("/*abcd  */", vec![Comment]);
        ast("/* ab- .-cd  */", vec![Comment]);
        ast("/* *** ** * ///* */", vec![Comment]);
    }

    #[test]
    fn whitespaces() {
        ast(" ", vec![WhiteSpace]);
        ast("      ", vec![WhiteSpace]);
    }

    // tests with skipping comments
    #[test]
    fn token_sequence() {
        ast("5 5 5 5", vec![Number, WhiteSpace, Number, WhiteSpace, Number, WhiteSpace, Number]);
        ast("/* 5 4 */-->", vec![Comment, CDC]);
        ast("/* 5 4 */ -->", vec![Comment, WhiteSpace, CDC]);
        ast("/* \"adaasd\" */ -->", vec![Comment, WhiteSpace, CDC]);
        ast("/* <!-- */ -->", vec![Comment, WhiteSpace, CDC]);
        ast("red-->", vec![Ident, DelimGreaterThanSign]);
        ast("@ import", vec![DelimCommercialAt, WhiteSpace, Ident]);
    }
}
