



// #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
// #[allow(non_camel_case_types)]
// #[repr(u16)]
// pub enum SyntaxKind {
//     // LEXER OUTPUT START
//     WHITESPACE,
//     // LEXER OUTPUT END

//     DECLARATIONS,
//     VARIABLE,
//     CUSTOM_PROPERTY_VALUE,
//     MEDIA_LIST,
//     NODE_LIST,

//     IDENTIFIER,
//     STYLESHEET,
//     SELECTOR,
//     SIMPLE_SELECTOR,
//     SELECTOR_INTERPOLATION,
//     SELECTOR_COMBINATOR,
//     SELECTOR_COMBINATOR_PARENT,
//     SELECTOR_COMBINATOR_SIBLING,
//     SELECTOR_COMBINATOR_ALL_SIBLINGS,
//     SELECTOR_COMBINATOR_SHADOW_PIERCING_DESCENDANT,
//     CLASS_SELECTOR,
//     IDENTIFIER_SELECTOR,
//     ELEMENT_NAME_SELECTOR,
//     PSEUDO_SELECTOR,
//     ATTRIBUTE_SELECTOR,
//     PROPERTY,
//     EXPRESSION,
//     BINARY_EXPRESSION,
//     TERM,
//     OPERATOR,
//     VALUE,
//     STRING_LITERAL,
//     URI_LITERAL,
//     ESCAPED_VALUE,
//     NUMERIC_VALUE,
//     HEX_COLOR_VALUE,
//     RATIO_VALUE,
//     VARIABLE_NAME,
//     PRIO,
//     INTERPOLATION,
//     EXTENDS_REFERENCE,
//     SELECTOR_PLACEHOLDER,
//     DEBUG,
//     IMPORT,
//     NAMESPACE,
//     RETURN_STATEMENT,
//     MEDIA_QUERY,
//     MEDIA_CONDITION,
//     MEDIA_FEATURE,
//     FUNCTION_PARAMETER,
//     FUNCTION_ARGUMENT,
//     AT_APPLY_RULE,
//     LIST_ENTRY,
//     SUPPORTS_CONDITION,
//     NAMESPACE_PREFIX,
//     GRID_LINE,
//     PLUGIN,
//     USE,
//     MODULE_CONFIGURATION,
//     FORWARD,
//     FORWARD_VISIBILITY,
//     MODULE,
//     UNICODE_RANGE,
//     LAYER_NAME_LIST,
//     LAYER_NAME,

//     // ----------
//     INVOCATION,
//         INVOCATION_INNER,
//         NORMY_INVOCATION,
//         FUNCTION,

//     // ---------
//     ABSTRACT_DECLARATION,
//         ABSTRACT_DECLARATION_INNER,
//         DECLARATION,
//             DECLARATION_INNER,
//             NORMY_DECLARATION,
//             CUSTOM_PROPERTY_DECLARATION,
//         VARIABLE_DECLARATION,

//     // -------------
//     BODY_DECLARATION,
//         BODY_DECLARATION_INNER,
//         RULE_SET,
//         CUSTOM_PROPERTY_SET,
//         IF_STATEMENT,
//         FOR_STATEMENT,
//         EACH_STATEMENT,
//         WHILE_STATEMENT,
//         ELSE_STATEMENT,
//         FUNCTION_DECLARATION,
//         VIEWPORT,
//         FONT_FACE,
//         NESTED_PROPERTIES,
//         KEYFRAME,
//         KEYFRAME_SELECTOR,
//         MEDIA,
//         SUPPORTS,
//         LAYER,
//         PROPERTY_AT_RULE,
//         DOCUMENT,
//         CONTAINER,
//         PAGE, 
//         PAGE_BOX_MARGIN_BOX,
//         MIXIN_CONTENT_DECLARATION,
//         MIXIN_DECLARATION,
//         UNKNOWN_AT_RULE,
    
//     //---

//     LESS_GUARD,
//     MIXIN_REFERENCE,
//     MIXIN_CONTENT_REFERENCE,

//     ROOT,   
// }

// use SyntaxKind::*;

// #[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
// pub enum CssLanguage {}

// impl From<SyntaxKind> for rowan::SyntaxKind {
//     fn from(kind: SyntaxKind) -> Self {
//         Self(kind as u16)
//     }
// }

// impl rowan::Language for CssLanguage {
//     type Kind = SyntaxKind;

//     fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
//         assert!(raw.0 <= ROOT as u16);
//         unsafe { std::mem::transmute::<u16, SyntaxKind>(raw.0)}
//     }

//     fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
//         kind.into()
//     }
// }

