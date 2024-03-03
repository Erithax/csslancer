use crate::parser::css_nodes::*;

#[derive(Debug, PartialEq)]
pub struct CssIssueType {
    pub rule: Rule,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    NumberExpected,
    ConditionExpected,
    RuleOrSelectorExpected,
    DotExpected,
    ColonExpected,
    SemiColonExpected,
    TermExpected,
    ExpressionExpected,
    OperatorExpected,
    IdentifierExpected,
    PercentageExpected,
    URIOrStringExpected,
    URIExpected,
    VariableNameExpected,
    VariableValueExpected,
    PropertyValueExpected,
    LeftCurlyExpected,
    RightCurlyExpected,
    LeftSquareBracketExpected,
    RightSquareBracketExpected,
    LeftParenthesisExpected,
    RightParenthesisExpected,
    CommaExpected,
    PageDirectiveOrDeclarationExpected,
    UnknownAtRule,
    UnknownKeyword,
    SelectorExpected,
    StringLiteralExpected,
    WhitespaceExpected,
    MediaQueryExpected,
    IdentifierOrWildcardExpected,
    WildcardExpected,
    IdentifierOrVariableExpected,
}

impl ParseError {
    pub fn issue(&self) -> CssIssueType {
        use ParseError::*;
        return CssIssueType {
            rule: match self {
                NumberExpected => Rule::new("css-numberexpected", "number expected"),
                ConditionExpected => Rule::new("css-conditionexpected", "condition expected"),
                RuleOrSelectorExpected => {
                    Rule::new("css-ruleorselectorexpected", "at-rule or selector expected")
                }
                DotExpected => Rule::new("css-dotexpected", "dot expected"),
                ColonExpected => Rule::new("css-colonexpected", "colon expected"),
                SemiColonExpected => Rule::new("css-semicolonexpected", "semi-colon expected"),
                TermExpected => Rule::new("css-termexpected", "term expected"),
                ExpressionExpected => Rule::new("css-expressionexpected", "expression expected"),
                OperatorExpected => Rule::new("css-operatorexpected", "operator expected"),
                IdentifierExpected => Rule::new("css-identifierexpected", "identifier expected"),
                PercentageExpected => Rule::new("css-percentageexpected", "percentage expected"),
                URIOrStringExpected => {
                    Rule::new("css-uriorstringexpected", "URI or string expected")
                }
                URIExpected => Rule::new("css-uriexpected", "URI expected"),
                VariableNameExpected => Rule::new("css-varnameexpected", "variable name expected"),
                VariableValueExpected => {
                    Rule::new("css-varvalueexpected", "variable value expected")
                }
                PropertyValueExpected => {
                    Rule::new("css-propertyvalueexpected", "property value expected")
                }
                LeftCurlyExpected => Rule::new("css-lcurlyexpected", "{{ expected"),
                RightCurlyExpected => Rule::new("css-rcurlyexpected", "}} expected"),
                LeftSquareBracketExpected => Rule::new("css-lbracketexpected", "[ expected"),
                RightSquareBracketExpected => Rule::new("css-rbracketexpected", "] expected"),
                LeftParenthesisExpected => Rule::new("css-lparentexpected", "( expected"),
                RightParenthesisExpected => Rule::new("css-rparentexpected", ") expected"),
                CommaExpected => Rule::new("css-commaexpected", "comma expected"),
                PageDirectiveOrDeclarationExpected => Rule::new(
                    "css-pagedirordeclexpected",
                    "page directive or declaration expected",
                ),
                UnknownAtRule => Rule::new("css-unknownatrule", "unknown at-rule"),
                UnknownKeyword => Rule::new("css-unknownkeyword", "unknown keyword"),
                SelectorExpected => Rule::new("css-selectorexpected", "selector expected"),
                StringLiteralExpected => {
                    Rule::new("css-stringliteralexpected", "string literal expected")
                }
                WhitespaceExpected => Rule::new("css-whitespaceexpected", "whitespace expected"),
                MediaQueryExpected => Rule::new("css-mediaqueryexpected", "media query expected"),
                IdentifierOrWildcardExpected => Rule::new(
                    "css-idorwildcardexpected",
                    "identifier or wildcard expected",
                ),
                WildcardExpected => Rule::new("css-wildcardexpected", "wildcard expected"),
                IdentifierOrVariableExpected => {
                    Rule::new("css-idorvarexpected", "identifier or variable expected")
                }
            },
        };
    }
}
