//! Types for tokens used for CSS syntax

use strum::EnumIter;
use tower_lsp::lsp_types::SemanticTokenType;

const BOOL: SemanticTokenType = SemanticTokenType::new("bool");
const PUNCTUATION: SemanticTokenType = SemanticTokenType::new("punct");
const ESCAPE: SemanticTokenType = SemanticTokenType::new("escape");
const LINK: SemanticTokenType = SemanticTokenType::new("link");
const RAW: SemanticTokenType = SemanticTokenType::new("raw");
const LABEL: SemanticTokenType = SemanticTokenType::new("label");
const REF: SemanticTokenType = SemanticTokenType::new("ref");
const HEADING: SemanticTokenType = SemanticTokenType::new("heading");
const LIST_MARKER: SemanticTokenType = SemanticTokenType::new("marker");
const LIST_TERM: SemanticTokenType = SemanticTokenType::new("term");
const DELIMITER: SemanticTokenType = SemanticTokenType::new("delim");
const INTERPOLATED: SemanticTokenType = SemanticTokenType::new("pol");
const ERROR: SemanticTokenType = SemanticTokenType::new("error");
const TEXT: SemanticTokenType = SemanticTokenType::new("text");

const ELEMENT_NAME: SemanticTokenType = SemanticTokenType::new("element_name");
const PROPERTY: SemanticTokenType = SemanticTokenType::new("property");
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
    Bool,
    Punctuation,
    Escape,
    Link,
    Raw,
    Label,
    Ref,
    Heading,
    ListMarker,
    ListTerm,
    Delimiter,
    Interpolated,
    Error,
    /// Any text in markup without a more specific token type, possible styled.
    ///
    /// We perform styling (like bold and italics) via modifiers. That means everything that should
    /// receive styling needs to be a token so we can apply a modifier to it. This token type is
    /// mostly for that, since text should usually not be specially styled.
    Text,

    ElementName,
    Property,
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
            Number => Self::NUMBER,
            Function => Self::FUNCTION,
            Decorator => Self::DECORATOR,
            Bool => BOOL,
            Punctuation => PUNCTUATION,
            Escape => ESCAPE,
            Link => LINK,
            Raw => RAW,
            Label => LABEL,
            Ref => REF,
            Heading => HEADING,
            ListMarker => LIST_MARKER,
            ListTerm => LIST_TERM,
            Delimiter => DELIMITER,
            Interpolated => INTERPOLATED,
            Error => ERROR,
            Text => TEXT,

            ElementName => ELEMENT_NAME,
            Property => PROPERTY,
            Important => IMPORTANT,
        }
    }
}
