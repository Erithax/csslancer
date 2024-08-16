//! Types for tokens used for CSS syntax

use strum::EnumIter;
use tower_lsp::lsp_types::SemanticTokenType;

const ESCAPE: SemanticTokenType = SemanticTokenType::new("escape");
const ESCAPE_INVALID: SemanticTokenType = SemanticTokenType::new("invalidEscape");
const URL: SemanticTokenType = SemanticTokenType::new("url");
const RAW: SemanticTokenType = SemanticTokenType::new("raw");
const PUNCTUATION: SemanticTokenType = SemanticTokenType::new("punct");
const BRACE: SemanticTokenType = SemanticTokenType::new("brace");
const BRACKET: SemanticTokenType = SemanticTokenType::new("bracket");
const PARENTHESIS: SemanticTokenType = SemanticTokenType::new("parenthesis");
const COLON: SemanticTokenType = SemanticTokenType::new("colon");
const SEMICOLON: SemanticTokenType = SemanticTokenType::new("semicolon");

const OPERATOR_ARITHMETICAL: SemanticTokenType = SemanticTokenType::new("arithmetical");
const OPERATOR_LOGICAL: SemanticTokenType = SemanticTokenType::new("logical");
const OPERATOR_COMPARATIVE: SemanticTokenType = SemanticTokenType::new("comparison");

const SELECTOR_COMBINATOR: SemanticTokenType = SemanticTokenType::new("selectorCombinator");
// const LABEL: SemanticTokenType = SemanticTokenType::new("label");
// const REF: SemanticTokenType = SemanticTokenType::new("ref");
// const HEADING: SemanticTokenType = SemanticTokenType::new("heading");
// const LIST_MARKER: SemanticTokenType = SemanticTokenType::new("marker");
// const LIST_TERM: SemanticTokenType = SemanticTokenType::new("term");
// const DELIMITER: SemanticTokenType = SemanticTokenType::new("delim");
// const INTERPOLATED: SemanticTokenType = SemanticTokenType::new("pol");
const ERROR: SemanticTokenType = SemanticTokenType::new("error");
const PROPERTY: SemanticTokenType = SemanticTokenType::new("property");
const TEXT: SemanticTokenType = SemanticTokenType::new("text");
const IDENTIFIER: SemanticTokenType = SemanticTokenType::new("identifier");

const IMPORTANT: SemanticTokenType = SemanticTokenType::new("important");

#[derive(Debug, Clone, Copy, EnumIter)]
#[repr(u32)]
pub enum SemTokenKind {
    // Standard LSP types
    Comment,
    String,
    Keyword,
    Operator,
    Number,
    Function,
    Decorator,
    // Custom types
    Punctuation,
    Escape,
    Url,
    Raw,
    // Label,
    // Ref,
    // Heading,
    // ListMarker,
    // ListTerm,
    // Delimiter,
    // Interpolated,
    Error,
    /// Any text in markup without a more specific token type, possible styled.
    ///
    /// We perform styling (like bold and italics) via modifiers. That means everything that should
    /// receive styling needs to be a token so we can apply a modifier to it. This token type is
    /// mostly for that, since text should usually not be specially styled.
    Text,

    //ElementName,
    OperatorArithmetical,
    OperatorLogical,
    OperatorComparative,
    SelectorCombinator,
    EscapeInvalid,

    Brace,
    Bracket,
    Parens,
    Colon,
    SemiColon,

    Property,
    Identifier,
    Important,
}

impl From<SemTokenKind> for SemanticTokenType {
    fn from(token_type: SemTokenKind) -> Self {
        use SemTokenKind::*;

        match token_type {
            Comment => Self::COMMENT,
            String => Self::STRING,
            Keyword => Self::KEYWORD,
            Operator => Self::OPERATOR,
            OperatorArithmetical => OPERATOR_ARITHMETICAL,
            OperatorLogical => OPERATOR_LOGICAL,
            OperatorComparative => OPERATOR_COMPARATIVE,
            Number => Self::NUMBER,
            Function => Self::FUNCTION,
            Decorator => Self::DECORATOR,
            Punctuation => PUNCTUATION,
            Brace => BRACE,
            Bracket => BRACKET,
            Parens => PARENTHESIS,
            Colon => COLON,
            SemiColon => SEMICOLON,

            Escape => ESCAPE,
            EscapeInvalid => ESCAPE_INVALID,
            Url => URL,
            Raw => RAW,
            Error => ERROR,
            SelectorCombinator => SELECTOR_COMBINATOR,
            Property => PROPERTY,
            Important => IMPORTANT,
            Identifier => IDENTIFIER,
            Text => TEXT,
        }
    }
}
