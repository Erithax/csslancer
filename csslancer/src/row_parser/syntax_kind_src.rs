//! Defines input for code generation process.

pub(crate) struct SyntaxKindsSrc<'a> {
    pub(crate) punct: &'a [(&'a str, &'a str)],
    pub(crate) tokens: &'a [&'a str],
    pub(crate) dimensions: &'a str,
    pub(crate) at_keywords: &'a str,
    pub(crate) contextual_ids: &'a [&'a str],
    pub(crate) contextual_funcs: &'a [&'a str],
    pub(crate) contextual_hash: &'a [&'a str],
    pub(crate) contextual_dims: &'a [&'a str],
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
        ("!", "EXCLAMATION"),
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
        ("!=", "NEQ"),
        ("-", "MINUS"),
        ("->", "THIN_ARROW"),
        ("<=", "LTEQ"),
        (">=", "GTEQ"),
        ("+=", "PLUSEQ"),
        ("-=", "MINUSEQ"),

        ("|=", "OPERATOR_DASHMATCH"),
        ("~=", "OPERATOR_INCLUDES"),
        ("^=", "OPERATOR_PREFIX"),
        ("$=", "OPERATOR_SUFFIX"),
        ("*=", "OPERATOR_SUBSTRING"),
        
        ("%=", "PERCENTEQ"),
        ("&&", "AMP2"),
        ("||", "PIPE2"),
        ("<<", "SHL"),
        (">>", "SHR"),
        ("<<=", "SHLEQ"),
        (">>=", "SHREQ"),
    ],
    // -ms-keyframes: https://github.com/CSSLint/csslint/issues/295
    // margin-at-rule: https://developer.mozilla.org/en-US/docs/Web/CSS/@page#margin_at-rules
    at_keywords: "
        unknown
        import 
        namespace 
        font-face 
        viewport -ms-viewport -o-viewport 
        keyframes -webkit-keyframes -moz-keyframes -o-keyframes
        property
        layer
        supports
        media
        page
        -moz-document
        container
        margin-at-rule", 
    dimensions: "unknown em ex px cm mm in pt pc deg rad grad ms s hz khz % fr dpi dpcm cqw cqh cqi cqb cqmin cqmax",
    tokens: &["error", "identifier", "string", "url", "bad_string", "bad_url", /*"ATKEYWORD",*/ "unrestricted_hash", "id_hash", "number", /*"DIMENSION",*/ "charset", "whitespace", "comment", "unicode_range", "function", "cdo", "cdc"],
    contextual_ids: &[
        "not",
        "and",
        "or",
        "screen",
        "only",
        "deep",
        "attrib_i",
        "attrib_s",
        "an_plus_b_syntax_an",
        "of",
        "important",
        "progid",
        "urlprefix",
        "valid_custom_prop",
    ],
    contextual_hash: &[
        "valid_hex",
    ],
    contextual_funcs: &[
        "layer",
        "supports",
        "style",
        "url",
    ],
    contextual_dims: &[
        "an_plus_b", // mapped on same SyntaxKind as contextual Id's an_plus_b
    ],
    css_nodes: stringify_many!{
        TODO
        UNDEFINED
        SOURCE_FILE

        // BODY DECLARATION
        RULE_SET
        PAGE
        PAGE_BOX_MARGIN_BOX
        VIEW_PORT
        DOCUMENT
        CUSTOM_PROPERTY_SET
        SUPPORTS
        FONT_FACE
        MEDIA
        LAYER
        KEYFRAME
        KEYFRAME_SELECTOR
        CONTAINER
        PROPERTY_AT_RULE
        UNKNOWN_AT_RULE
        // --
        // SELECTOR
        SELECTOR
        SIMPLE_SELECTOR
        SELECTOR_INTERPOLATION
        SELECTOR_COMBINATOR
        SELECTOR_COMBINATOR_PARENT
        SELECTOR_COMBINATOR_SIBLING
        SELECTOR_COMBINATOR_ALL_SIBLINGS
        SELECTOR_COMBINATOR_SHADOW_PIERCING_DESCENDANT
        SELECTOR_CLASS
        SELECTOR_IDENTIFIER
        SELECTOR_ELEMENT_NAME
        SELECTOR_PSEUDO
        SELECTOR_ATTRIBUTE
        // --

        // ABSTRACT DECLARATION
        ABSTRACT_DECLARATION
        DECLARATION
        DECLARATION_BASIC 
        DECLARATION_CUSTOM_PROPERTY
        DECLARATION_XCSS_VARIABLE
        // -- 

        DECLARATIONS
        PROPERTY
        EXPRESSION
        BINARY_EXPRESSION
        TERM
        OPERATOR
        STRING_LITERAL
        URI_LITERAL
        NUMERIC_VALUE
        HEX_COLOR_VALUE
        RATIO_VALUE
        PRIO
        IMPORT
        NAMESPACE
        MEDIA_QUERY
        MEDIA_CONDITION
        MEDIA_FEATURE
        FUNCTION_WITH_ARGS
        FUNCTION_ARGUMENT
        SUPPORTS_CONDITION
        NAMESPACE_PREFIX
        GRID_LINE
        LAYER_NAME_LIST
        LAYER_NAME

        //VALUE
        //INVOCATION
        //AT_APPLY_RULE
    },
    xcss_nodes: stringify_many!{
        EXTENDS_REFERENCE
        FUNCTION_PARAMETER
        MIXIN_DECLARATION
        MIXIN_REFERENCE
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