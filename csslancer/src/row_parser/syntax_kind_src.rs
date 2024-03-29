//! Defines input for code generation process.

pub(crate) struct SyntaxKindsSrc<'a> {
    pub(crate) punct: &'a [(&'a str, &'a str)],
    pub(crate) tokens: &'a [&'a str],
    pub(crate) dimensions: &'a str,
    pub(crate) css_nodes: &'a [&'a str],
    pub(crate) xcss_nodes: &'a [&'a str], // in scss and less
    pub(crate) scss_nodes: &'a [&'a str],
    pub(crate) less_nodes: &'a [&'a str],

}

macro_rules! stringify_many {
    ($($i:ident)*) => {
        
        &[$(stringify!($i),)*]
    }
}

pub(crate) const SYNTAX_KINDS_SRC: SyntaxKindsSrc<'_> = SyntaxKindsSrc {
    punct: &[
        (";", "SEMICOLON"),
        (",", "COMMA"),
        ("(", "L_PAREN"),
        (")", "R_PAREN"),
        ("{", "L_CURLY"),
        ("}", "R_CURLY"),
        ("[", "L_BRACK"),
        ("]", "R_BRACK"),
        ("<", "L_ANGLE"),
        (">", "R_ANGLE"),
        ("@", "AT"),
        ("#", "POUND"),
        ("~", "TILDE"),
        ("?", "QUESTION"),
        ("$", "DOLLAR"),
        ("&", "AMP"),
        ("|", "PIPE"),
        ("+", "PLUS"),
        ("*", "STAR"),
        ("/", "SLASH"),
        ("^", "CARET"),
        ("%", "PERCENT"),
        ("_", "UNDERSCORE"),
        (".", "DOT"),
        ("..", "DOT2"),
        ("...", "DOT3"),
        ("..=", "DOT2EQ"),
        (":", "COLON"),
        ("::", "COLON2"),
        ("=", "EQ"),
        ("==", "EQ2"),
        ("=>", "FAT_ARROW"),
        ("!", "BANG"),
        ("!=", "NEQ"),
        ("-", "MINUS"),
        ("->", "THIN_ARROW"),
        ("<=", "LTEQ"),
        (">=", "GTEQ"),
        ("+=", "PLUSEQ"),
        ("-=", "MINUSEQ"),
        ("|=", "PIPEEQ"),
        ("&=", "AMPEQ"),
        ("^=", "CARETEQ"),
        ("/=", "SLASHEQ"),
        ("*=", "STAREQ"),
        ("%=", "PERCENTEQ"),
        ("&&", "AMP2"),
        ("||", "PIPE2"),
        ("<<", "SHL"),
        (">>", "SHR"),
        ("<<=", "SHLEQ"),
        (">>=", "SHREQ"),
    ],
    dimensions: "em ex px cm mm in pt pc deg rad grad ms s hz khz % fr dpi dpcm cqw cqh cqi cqb cqmin cqmax",
    tokens: &["ERROR", "IDENT", "WHITESPACE", "COMMENT"],
    css_nodes: stringify_many!{
        SOURCE_FILE
        RULE_SET
        SELECTOR
        SIMPLE_SELECTOR
        SELECTOR_INTERPOLATION
        SELECTOR_COMBINATOR
        SELECTOR_COMBINATOR_PARENT
        SELECTOR_COMBINATOR_SIBLING
        SELECTOR_COMBINATOR_ALL_SIBLINGS
        SELECTOR_COMBINATOR_SHADOW_PIERCING_DESCENDANT
        PAGE
        PAGE_BOX_MARGIN_BOX
        SELECTOR_CLASS
        SELECTOR_IDENTIFIER
        SELECTOR_ELEMENT_NAME
        SELECTOR_PSEUDO
        SELECTOR_ATTRIBUTE
        DECLARATION
        DECLARATIONS
        PROPERTY
        EXPRESSION
        BINARY_EXPRESSION
        TERM
        OPERATOR
        STRING_LITERAL
        URI_LITERAL
        FUNCTION
        NUMERIC_VALUE
        HEX_COLOR_VALUE
        RATIO_VALUE
        PRIO
        MEDIA
        KEYFRAME
        FONT_FACE
        IMPORT
        NAMESPACE
        MEDIA_QUERY
        MEDIA_CONDITION
        MEDIA_FEATURE
        FUNCTION_ARGUMENT
        KEYFRAME_SELECTOR
        VIEWPORT
        DOCUMENT
        CUSTOM_PROPERTY_DECLARATION
        CUSTOM_PROPERTY_SET
        SUPPORTS
        SUPPORTS_CONDITION
        NAMESPACE_PREFIX
        GRID_LINE
        UNKNOWN_AT_RULE
        UNICODE_RANGE
        LAYER
        LAYER_NAME_LIST
        PROPERTY_AT_RULE
        CONTAINER
        //VALUE
        //INVOCATION
        //AT_APPLY_RULE
    },
    xcss_nodes: stringify_many!{
        EXTENDS_REFERENCE
        FUNCTION_PARAMETER
        MIXIN_DECLARATION
        MIXIN_REFERENCE
        VARIABLE_DECLARATION
    },
    scss_nodes: stringify_many!{
        NESTED_PROPERTIES
        SELECTOR_PLACEHOLDER
        DEBUG
        IF_STATEMENT
        ELSE_STATEMENT
        FOR_STATEMENT
        EACH_STATEMENT
        WHILE_STATEMENT
        RETURN_STATEMENT
        FUNCTION_DECLARATION
        LIST_ENTRY
        USE
        MODULE
        MODULE_CONFIGURATION
        FORWARD
        FORWARD_VISIBILITY
        MIXIN_CONTENT_REFERENCE
        MIXIN_CONTENT_DECLARATION
        VARIABLE_NAME

    },
    less_nodes: stringify_many!{
        ESCAPED_VALUE
        INTERPOLATION
        PLUGIN
    },
};

#[derive(Default, Debug)]
pub(crate) struct AstSrc {
    pub(crate) tokens: Vec<String>,
    pub(crate) nodes: Vec<AstNodeSrc>,
    pub(crate) enums: Vec<AstEnumSrc>,
}

#[derive(Debug)]
pub(crate) struct AstNodeSrc {
    pub(crate) doc: Vec<String>,
    pub(crate) name: String,
    pub(crate) traits: Vec<String>,
    pub(crate) fields: Vec<Field>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Field {
    Token(String),
    Node { name: String, ty: String, cardinality: Cardinality },
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Cardinality {
    Optional,
    Many,
}

#[derive(Debug)]
pub(crate) struct AstEnumSrc {
    pub(crate) doc: Vec<String>,
    pub(crate) name: String,
    pub(crate) traits: Vec<String>,
    pub(crate) variants: Vec<String>,
}