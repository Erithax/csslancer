#[cfg(test)]
mod css_grammar_test {
    use super::super::{
        parser::Parser,
        syntax_kind_gen::SyntaxKind,
        Parse,
        CssLanguage,
        parse_error::ParseError::{self, *},
    };
    use csslancer_macro::{assert_parse_error, assert_parse_node};
    use rowan::{SyntaxNode, GreenNode};
    use std::marker::PhantomData;

    fn assert_node<F: Fn(&mut Parser) -> Option<Result<(), ()>>>(text: &str, f: F) -> GreenNode {
        println!("text: {text}");
        let (success, (green, errors)) = super::super::must_parse_text_as_fn(text, f);
        let root = SyntaxNode::<CssLanguage>::new_root(green.clone());

        assert!(success, "did not parse expected node from fn from text `{text}`");
        assert!(errors.len() == 0, "unexpected errors while parsing `{text}`: {errors:?}");
        
        green
    }

    fn assert_no_node<F: Fn(&mut Parser) -> Option<Result<(), ()>>>(text: &str, f: F) {
        println!("text: {text}");
        let input_text_len = text.len();
        let (success, (green, errors)) = super::super::must_parse_text_as_fn(text, f);
        let root = SyntaxNode::<CssLanguage>::new_root(green.clone());

        assert!(!success || <rowan::TextSize as Into<usize>>::into(green.text_len()) != text.len(), "did not expected succesfully parsed node from text `{text}`. 
            \n instead found tree spanning whole text with errors `{errors:?}`.
        ");    
    }

    fn assert_error<F: Fn(&mut Parser) -> Option<Result<(), ()>>>(text: &str, f: F, expected_error: ParseError) -> GreenNode {
        println!("text: {text}");
        let (success, (green, errors)) = super::super::must_parse_text_as_fn(text, f);
        SyntaxNode::<CssLanguage>::new_root(green.clone());

        assert!(success, "did not parse expected node from fn from text `{text}`");
        assert!(errors.len() != 0, "expected parse error `{expected_error:?}`, but none were found, while parsing `{text}`");
        assert_eq!(expected_error.issue().desc, errors[0].to_string(), "expected first error `{}`, but encountered errors (in order) `{:?}`", expected_error.issue().desc, errors);
        
        green
    }

    #[test]
    fn stylesheet() {
        let f = |p: &mut Parser| Some(Ok(p.parse_source_file()));
        assert_node("@charset \"demo\" ;", f);
        assert_node("body { margin: 0px; padding: 3em, 6em; }", f);
        assert_node("--> <!--", f);
        assert_node("", f);
        assert_node("<!-- --> @import \"string\"; <!-- -->", f);
        assert_node("@media asdsa { } <!-- --> <!-- -->", f);
        assert_node("@media screen, projection { }", f);
        assert_node(
            "@media screen and (max-width: 400px) {  @-ms-viewport { width: 320px; }}",
            f
        );
        assert_node(
            "@-ms-viewport { width: 320px; height: 768px; }",
            f
        );
        assert_node("#boo, far {} \n.far boo {}", f);
        assert_node("@-moz-keyframes darkWordHighlight { from { background-color: inherit; } to { background-color: rgba(83, 83, 83, 0.7); } }", f);
        assert_node("@page { margin: 2.5cm; }", f);
        assert_node(
            r#"@font-face { font-family: "Example Font"; }"#,
            f
        );
        assert_node(
            r#"@namespace "http://www.w3.org/1999/xhtml";"#,
            f
        );
        assert_node("@namespace pref url(http://test);", f);
        assert_node("@-moz-document url(http://test), url-prefix(http://www.w3.org/Style/) { body { color: purple; background: yellow; } }", f);
        assert_node(r#"E E[foo] E[foo="bar"] E[foo~="bar"] E[foo^="bar"] E[foo$="bar"] E[foo*="bar"] E[foo|="en"] {}"#, f);
        assert_node(r#"input[type="submit"] {}"#, f);
        assert_node("E:root E:nth-child(n) E:nth-last-child(n) E:nth-of-type(n) E:nth-last-of-type(n) E:first-child E:last-child {}", f);
        assert_node("E:first-of-type E:last-of-type E:only-child E:only-of-type E:empty E:link E:visited E:active E:hover E:focus E:target E:lang(fr) E:enabled E:disabled E:checked {}", f);
        assert_node(
            "E::first-line E::first-letter E::before E::after {}",
            f
        );
        assert_node("E.warning E#myid E:not(s) {}", f);
        assert_error("@namespace;", f, URIExpected);
        assert_error(
            "@namespace url(http://test)",
            f,
            SemiColonExpected
        );
        // assert_error("@charset;", f, IdentifierExpected);  charset is parsed in tokenizer, will err in later stage because unknown at rule
        //assert_error("@charset 'utf8'", f, SemiColonExpected);
    }

    #[test]
    fn stylesheet_graceful_unknown_rules() {
        let f = |p: &mut Parser| Some(Ok(p.parse_source_file()));
        assert_node("@unknown-rule;", f);
        assert_node("@unknown-rule 'foo';", f);
        assert_node("@unknown-rule (foo) {}", f);
        assert_node("@unknown-rule (foo) { .bar {} }", f);
        assert_node("@mskeyframes darkWordHighlight { from { background-color: inherit; } to { background-color: rgba(83, 83, 83, 0.7); } }", f);
        assert_node("foo { @unknown-rule; }", f);

        assert_error(
            "@unknown-rule (;",
            f,
            RightParenthesisExpected
        );
        assert_error(
            "@unknown-rule [foo",
            f,
            RightSquareBracketExpected
        );
        assert_error(
            "@unknown-rule { [foo }",
            f,
            RightSquareBracketExpected
        );
        assert_error("@unknown-rule (foo) {", f, RightCurlyExpected);
        assert_error(
            "@unknown-rule (foo) { .bar {}",
            f,
            RightCurlyExpected
        );
    }

    #[test]
    fn stylesheet_unknown_rules_node_proper_end() {
        // Microsoft/vscode#53159
        let tree = assert_node("@unknown-rule (foo) {} .foo {}", |p: &mut Parser| Some(Ok(p.parse_source_file())));

        assert_eq!(SyntaxKind::SOURCE_FILE as u16, tree.kind().0);

        assert_eq!(SyntaxKind::UNKNOWN_AT_RULE as u16, tree.children().next().unwrap().kind().0);

        // microsoft/vscode-css-languageservice#237
        assert_node(
            ".foo { @apply p-4 bg-neutral-50; min-height: var(--space-14); }",
            |p: &mut Parser| Some(Ok(p.parse_source_file()))
        );
    }

    #[test]
    fn stylesheet_error() {
        let f = |p: &mut Parser| Some(Ok(p.parse_source_file()));
        assert_error(
            "#boo, far } \n.far boo {}",
            f,
            LeftCurlyExpected
        );
        assert_error(
            "#boo, far { far: 43px; \n.far boo {}",
            f,
            RightCurlyExpected
        );
        assert_error(
            r#"- @import "foo";"#,
            f,
            RuleOrSelectorExpected
        );
    }

    #[test]
    fn font_face() {
        let f = |p: &mut Parser| p.parse_font_face_opt();
        assert_node("@font-face {}", f);
        assert_node("@font-face { src: url(http://test) }", f);
        assert_node(
            "@font-face { font-style: normal; font-stretch: normal; }",
            f
        );
        assert_node("@font-face { unicode-range: U+0021-007F }", f);
        // assert_error(
        //     "@font-face { font-style: normal font-stretch: normal; }",
        //     f,
        //     SemiColonExpected
        // );
    }

    #[test]
    fn keyframe_selector() {
        let f = |p: &mut Parser| p.parse_keyframe_selector_opt();
        assert_node("from {}", f);
        assert_node("to {}", f);
        assert_node("0% {}", f);
        assert_node("10% {}", f);
        assert_node("cover 10% {}", f);
        assert_node("100000% {}", f);
        assert_node("from { width: 100% }", f);
        assert_node("from { width: 100%; to: 10px; }", f);
        assert_node("from, to { width: 10px; }", f);
        assert_node("10%, to { width: 10px; }", f);
        assert_node("from, 20% { width: 10px; }", f);
        assert_node("10%, 20% { width: 10px; }", f);
        assert_node("cover 10% {}", f);
        assert_node("cover 10%, exit 20% {}", f);
        assert_node("10%, exit 20% {}", f);
        assert_node("from, exit 20% {}", f);
        assert_node("cover 10%, to {}", f);
        assert_node("cover 10%, 20% {}", f);
    }

    #[test]
    fn at_keyframe() {
        let f = |p: &mut Parser| p.parse_keyframe_opt();
        assert_node("@keyframes name {}", f);
        assert_node("@-webkit-keyframes name {}", f);
        assert_node("@-o-keyframes name {}", f);
        assert_node("@-moz-keyframes name {}", f);
        assert_node("@keyframes name { from {} to {}}", f);
        assert_node("@keyframes name { from {} 80% {} 100% {}}", f);
        assert_node(
            "@keyframes name { from { top: 0px; } 80% { top: 100px; } 100% { top: 50px; }}",
            f
        );
        assert_node(
            "@keyframes name { from { top: 0px; } 70%, 80% { top: 100px; } 100% { top: 50px; }}",
            f
        );
        assert_node(
            "@keyframes name { from { top: 0px; left: 1px; right: 2px }}",
            f
        );
        assert_node(
            "@keyframes name { exit 50% { top: 0px; left: 1px; right: 2px }}",
            f
        );
        // assert_error(
        //     "@keyframes name { from { top: 0px; left: 1px, right: 2px }}",
        //     f,
        //     SemiColonExpected
        // );
        assert_error("@keyframes )", f, IdentifierExpected);
        assert_error(
            "@keyframes name { { top: 0px; } }",
            f,
            RightCurlyExpected
        );
        assert_error("@keyframes name { from, #123", f, PercentageExpected);
        assert_error(
            "@keyframes name { 10% from { top: 0px; } }",
            f,
            LeftCurlyExpected
        );
        assert_error(
            "@keyframes name { 10% 20% { top: 0px; } }",
            f,
            LeftCurlyExpected
        );
        assert_error(
            "@keyframes name { from to { top: 0px; } }",
            f,
            LeftCurlyExpected
        );
    }

    #[test]
    fn at_property() {
        assert_node(
            "@property --my-color { syntax: '<color>'; inherits: false; initial-value: #c0ffee; }",
            |p: &mut Parser| Some(Ok(p.parse_source_file()))
        );
        assert_error("@property  {  }", |p: &mut Parser| Some(Ok(p.parse_source_file())), IdentifierExpected);
    }

    #[test]
    fn at_container() {
        assert_node(
            "@container (width <= 150px) { #inner { background-color: skyblue; }}",
            |p: &mut Parser| Some(Ok(p.parse_source_file()))
        );
        assert_node(
            "@container card (inline-size > 30em) and style(--responsive: true) { }",
            |p: &mut Parser| Some(Ok(p.parse_source_file()))
        );
        assert_node(
            "@container card (inline-size > 30em) { @container style(--responsive: true) {} }",
            |p: &mut Parser| Some(Ok(p.parse_source_file()))
        );
    }

    #[test]
    fn at_container_query_len_units() {
        assert_node(
            "@container (min-width: 700px) { .card h2 { font-size: max(1.5em, 1.23em + 2cqi); } }",
            |p: &mut Parser| Some(Ok(p.parse_source_file()))
        );
    }

    #[test]
    fn at_import() {
        let f = |p: &mut Parser| p.parse_import_opt();
        let src_f = |p: &mut Parser| Some(Ok(p.parse_source_file()));

        assert_node(r#"@import "asdasdsa""#, f);
        assert_node(r#"@ImPort "asdsadsa""#, f);
        assert_node(r#"@import "asdasd" dsfsdf"#, f);
        assert_node(r#"@import "foo";"#, src_f);
        assert_node(
            "@import url(/css/screen.css) screen, projection;",
            src_f
        );
        assert_node(
            "@import url('landscape.css') screen and (orientation:landscape);",
            src_f
        );
        assert_node(
            r#"@import url("/inc/Styles/full.css") (min-width: 940px);"#,
            src_f
        );
        assert_node(
            "@import url(style.css) screen and (min-width:600px);",
            src_f
        );
        assert_node(
            r#"@import url("./700.css") only screen and (max-width: 700px);"#,
            src_f
        );

        assert_node(r#"@import url("override.css") layer;"#, src_f);
        assert_node(
            r#"@import url("tabs.css") layer(framework.component);"#,
            src_f
        );

        assert_node(
            r#"@import "mystyle.css" supports(display: flex);"#,
            src_f
        );

        assert_node(
            r#"@import url("narrow.css") supports(display: flex) handheld and (max-width: 400px);"#,
            src_f
        );
        assert_node(
            r#"@import url("fallback-layout.css") supports(not (display: flex));"#,
            src_f
        );

        assert_error("@import", src_f, URIOrStringExpected);
    }

    #[test]
    fn at_supports() {
        let f = |p: &mut Parser| p.parse_supports_opt(false);
        assert_node(
            "@supports ( display: flexbox ) { body { display: flexbox } }",
            f
        );
        assert_node("@supports not (display: flexbox) { .outline { box-shadow: 2px 2px 2px black; /* unprefixed last */ } }", f);
        assert_node("@supports ( box-shadow: 2px 2px 2px black ) or ( -moz-box-shadow: 2px 2px 2px black ) or ( -webkit-box-shadow: 2px 2px 2px black ) { }", f);
        assert_node("@supports ((transition-property: color) or (animation-name: foo)) and (transform: rotate(10deg)) { }", f);
        assert_node("@supports ((display: flexbox)) { }", f);
        assert_node(
            "@supports (display: flexbox !important) { }",
            f
        );
        assert_node(
            "@supports (grid-area: auto) { @media screen and (min-width: 768px) { .me { } } }",
            f
        );
        assert_node("@supports (column-width: 1rem) OR (-moz-column-width: 1rem) OR (-webkit-column-width: 1rem) oR (-x-column-width: 1rem) { }", f); // #49288
        assert_node("@supports not (--validValue: , 0 ) {}", f); // #82178
        assert_error("@supports (transition-property: color) or (animation-name: foo) and (transform: rotate(10deg)) { }", f, LeftCurlyExpected);
        assert_error(
            "@supports display: flexbox { }",
            f,
            LeftParenthesisExpected
        );
    }

    #[test]
    fn at_media() {
        let f = |p: &mut Parser| p.parse_media_opt(false);
        assert_node("@media asdsa { }", f);
        assert_node("@meDia sadd{}  ", f);
        assert_node("@media somename, othername2 { }", f);
        assert_node("@media only screen and (max-width:850px) { }", f);
        assert_node("@media only screen and (max-width:850px) { }", f);
        assert_node("@media all and (min-width:500px) { }", f);
        assert_node(
            "@media screen and (color), projection and (color) { }",
            f
        );
        assert_node(
            "@media not screen and (device-aspect-ratio: 16/9) { }",
            f
        );
        assert_node(
            "@media print and (min-resolution: 300dpi) { }",
            f
        );
        assert_node(
            "@media print and (min-resolution: 118dpcm) { }",
            f
        );
        assert_node(
            "@media print { @page { margin: 10% } blockquote, pre { page-break-inside: avoid } }",
            f
        );
        assert_node("@media print { body:before { } }", f);
        assert_node(
            "@media not (-moz-os-version: windows-win7) { }",
            f
        );
        assert_node(
            "@media not (not (-moz-os-version: windows-win7)) { }",
            f
        );
        assert_node("@media (height > 600px) { }", f);
        assert_node("@media (height < 600px) { }", f);
        assert_node("@media (height <= 600px) { }", f);
        assert_node("@media (400px <= width <= 700px) { }", f);
        assert_node("@media (400px >= width >= 700px) { }", f);
        assert_node(
            "@media screen and (750px <= width < 900px) { }",
            f
        );
        assert_error(
            "@media somename othername2 { }",
            f,
            LeftCurlyExpected
        );
        assert_error("@media not, screen { }", f, MediaQueryExpected);
        assert_error(
            "@media not screen and foo { }",
            f,
            LeftParenthesisExpected
        );
        assert_error(
            "@media not screen and () { }",
            f,
            IdentifierExpected
        );
        assert_error(
            "@media not screen and (color:) { }",
            f,
            TermExpected
        );
        assert_error(
            "@media not screen and (color:#234567 { }",
            f,
            RightParenthesisExpected
        );
    }

    #[test]
    fn media_list() {
        let f = |p: &mut Parser| Some(p.parse_media_query_list());
        assert_node("somename", f);
        assert_node("somename, othername", f);
        assert_node("not all and (monochrome)", f);
    }

    #[test]
    fn medium() {
        let f = |p: &mut Parser| p.parse_medium_opt();

        assert_node("somename", f);
        assert_node("-asdas", f);
        assert_node("-asda34s", f);
    }

    #[test]
    fn at_page() {
        let f = |p: &mut Parser| p.parse_page();

        assert_node("@page : name{ }", f);
        assert_node("@page :left, :right { }", f);
        assert_node("@page : name{ some : \"asdas\" }", f);
        assert_node("@page : name{ some : \"asdas\" !important }", f);
        assert_node(
            "@page : name{ some : \"asdas\" !important; some : \"asdas\" !important }",
            f
        );
        assert_node("@page rotated { size : landscape }", f);
        assert_node("@page :left { margin-left: 4cm; margin-right: 3cm; }", f);
        assert_node(
            "@page {  @top-right-corner { content: url(foo.png); border: solid green; } }",
            f
        );
        assert_node("@page {  @top-left-corner { content: \" \"; border: solid green; } @bottom-right-corner { content: counter(page); border: solid green; } }", f);
        assert_error(
            "@page {  @top-left-corner foo { content: \" \"; border: solid green; } }",
            f,
            LeftCurlyExpected
        );
        assert_error(r#"@page {  @XY foo { content: " "; border: solid green; } }"#, f, UnknownAtRule);
        // assert_error(
        //     "@page :left { margin-left: 4cm margin-right: 3cm; }",
        //     f,
        //     SemiColonExpected
        // );
        assert_error("@page : { }", f, IdentifierExpected);
        assert_error("@page :left, { }", f, IdentifierExpected);
    }

    #[test]
    fn at_layer() {
        let f = |p: &mut Parser| p.parse_layer_opt(false);
        assert_node(
            "@layer utilities { .padding-sm { padding: .5rem; } }",
            f
        );
        assert_node("@layer utilities;", f);
        assert_node("@layer theme, layout, utilities;", f);
        assert_node(
            "@layer utilities { p { margin-block: 1rem; } }",
            f
        );
        assert_node("@layer framework { @layer layout { } }", f);
        assert_node(
            "@layer framework.layout { @keyframes slide-left {} }",
            f
        );

        assert_node(
            "@media (min-width: 30em) { @layer layout { } }",
            |p: &mut Parser| Some(Ok(p.parse_source_file()))
        );

        assert_error("@layer theme layout {  }", f, SemiColonExpected);
        assert_error("@layer theme, layout {  }", f, SemiColonExpected);
        assert_error(
            "@layer framework .layout {  }",
            f,
            SemiColonExpected
        );
        assert_error(
            "@layer framework. layout {  }",
            f,
            IdentifierExpected
        );
    }

    #[test]
    fn operator() {
        let f = |p: &mut Parser| p.parse_operator_opt().map(|_| Ok(()));
        assert_node("/", f);
        assert_node("*", f);
        assert_node("+", f);
        assert_node("-", f);
    }

    #[test]
    fn combinator() {
        let f = |p: &mut Parser| p.parse_combinator_opt().map(|_| Ok(()));
        assert_node("+", f);
        assert_node("+  ", f);
        assert_node(">  ", f);
        assert_node(">", f);
        assert_node(">>>", f);
        assert_node("/deep/", f);
        assert_node(":host >>> .data-table { width: 100%; }", |p: &mut Parser| Some(Ok(p.parse_source_file())));
        assert_error(
            ":host >> .data-table { width: 100%; }",
            |p: &mut Parser| Some(Ok(p.parse_source_file())),
            LeftCurlyExpected
        );
    }

    #[test]
    fn unary_operator() {
        let f = |p: &mut Parser| p.parse_unary_operator_opt();
        assert_node("-", f);
        assert_node("+", f);
    }

    #[test]
    fn property() {
        let f = |p: &mut Parser| p.parse_property_opt();

        assert_node("asdsa", f);
        assert_node("asdsa334", f);

        assert_node("--color", f);
        assert_node("--primary-font", f);
        assert_node("-color", f);
        assert_node("somevar", f);
        assert_node("some--let", f);
        assert_node("somevar--", f);
    }

    #[test]
    fn ruleset() {
        let f = |p: &mut Parser| p.parse_rule_set_opt(false);
        assert_node("name{ }", f);
        assert_node("	name\n{ some : \"asdas\" }", f);
        assert_node("		name{ some : \"asdas\" !important }", f);
        assert_node(
            "name{ \n some : \"asdas\" !important; some : \"asdas\" }",
            f
        );
        assert_node("* {}", f);
        assert_node(".far{}", f);
        assert_node("boo {}", f);
        assert_node(".far #boo {}", f);
        assert_node("boo { prop: value }", f);
        assert_node("boo { prop: value; }", f);
        assert_node("boo { prop: value; prop: value }", f);
        assert_node("boo { prop: value; prop: value; }", f);
        //assert_node("boo {--minimal: }", f);
        assert_node("boo {--minimal: ;}", f);
        assert_node("boo {--normal-text: red yellow green}", f);
        assert_node("boo {--normal-text: red yellow green;}", f);
        assert_node("boo {--important: red !important;}", f);
        assert_node("boo {--nested: {color: green;}}", f);
        assert_node("boo {--parens: this()is()ok()}", f);
        assert_node("boo {--squares: this[]is[]ok[]too[]}", f);
        assert_node("boo {--combined: ([{{[]()()}[]{}}])()}", f);
        assert_node(
            "boo {--weird-inside-delims: {color: green;;;;;;!important;;}}",
            f
        );
        assert_node("boo {--validValue: , 0 0}", f);
        assert_node("boo {--validValue: , 0 0;}", f);
        assert_error("boo, { }", f, SelectorExpected);
    }

    #[test]
    fn ruleset_panic() {
        let f = |p: &mut Parser| p.parse_rule_set_opt(false);

        // no bueno assert_node("boo { : value }", f);
        assert_error("boo { prop: ; }", f, PropertyValueExpected);
        assert_error("boo { prop }", f, ColonExpected);
        assert_error(
            "boo { prop: ; far: 12em; }",
            f,
            PropertyValueExpected
        );
        //no bueno assert_node("boo { prop: ; 1ar: 12em; }", f);

        assert_error(
            "boo { --too-minimal:}",
            f,
            PropertyValueExpected
        );
        assert_error(
            "boo { --unterminated: ",
            f,
            RightCurlyExpected
        );
        // assert_error(
        //     "boo { --double-important: red !important !important;}",
        //     f,
        //     SemiColonExpected
        // );
        assert_error(
            "boo {--unbalanced-curlys: {{color: green;}}",
            f,
            RightCurlyExpected
        );
        assert_error(
            "boo {--unbalanced-parens: not(()cool;}",
            f,
            LeftCurlyExpected
        );
        assert_error(
            "boo {--unbalanced-parens: not)()(cool;}",
            f,
            LeftParenthesisExpected
        );
        assert_error(
            "boo {--unbalanced-brackets: not[[]valid;}",
            f,
            LeftCurlyExpected
        );
        assert_error(
            "boo {--unbalanced-brackets: not][][valid;}",
            f,
            LeftSquareBracketExpected
        );
    }

    #[test]
    fn nested_ruleset() {
        let f = |p: &mut Parser| p.parse_rule_set_opt(false);

        assert_node(
            ".foo { color: red; input { color: blue; } }",
            f
        );
        assert_node(
            ".foo { color: red; :focus { color: blue; } }",
            f
        );
        assert_node(
            ".foo { color: red; .bar { color: blue; } }",
            f
        );
        assert_node(
            ".foo { color: red; &:hover { color: blue; } }",
            f
        );
        assert_node(
            ".foo { color: red; + .bar { color: blue; } }",
            f
        );
        assert_node(
            ".foo { color: red; foo:hover { color: blue }; }",
            f
        );
        assert_node(
            ".foo { color: red; @media screen { color: blue }; }",
            f
        );

        // Top level curly braces are allowed in declaration values if they are for a custom property.
        assert_node(".foo { --foo: {}; }", f);
        // Top level curly braces are not allowed in declaration values.
        assert_error(".foo { foo: {}; }", f, PropertyValueExpected);
    }

    #[test]
    fn nested_ruleset_2() {
        let f = |p: &mut Parser| p.parse_rule_set_opt(false);

        assert_node(".foo { .parent & { color: blue; } }", f);
        assert_node(
            ".foo { color: red; & > .bar, > .baz { color: blue; } }",
            f
        );
        assert_node(
            ".foo { & .bar & .baz & .qux { color: blue; } }",
            f
        );
        assert_node(
            ".foo { color: red; :not(&) { color: blue; }; + .bar + & { color: green; } }",
            f
        );
        assert_node(
            ".foo { color: red; & { color: blue; } && { color: green; } }",
            f
        );
        assert_node(
            ".foo { & :is(.bar, &.baz) { color: red; } }",
            f
        );
        assert_node("figure { > figcaption { background: hsl(0 0% 0% / 50%); > p {  font-size: .9rem; } } }", f);
        assert_node(
            "@layer base { html { & body { min-block-size: 100%; } } }",
            |p: &mut Parser| Some(Ok(p.parse_source_file()))
        );
    }

    #[test]
    fn selector() {
        let f = |p: &mut Parser| p.parse_selector_opt(false);

        assert_node("asdsa", f);
        assert_node("asdsa + asdas", f);
        assert_node("asdsa + asdas + name", f);
        assert_node("asdsa + asdas + name", f);
        assert_node("name #id#anotherid", f);
        assert_node("name.far .boo", f);
        assert_node("name .name .zweitername", f);
        assert_node("*", f);
        assert_node("#id", f);
        assert_node("far.boo", f);
        assert_node("::slotted(div)::after", f); // 35076
    }

    #[test]
    fn simple_selector() {
        let f = |p: &mut Parser| p.parse_simple_selector().map(|_| Ok(()));

        assert_node("name", f);
        assert_node("#id#anotherid", f);
        assert_node("name.far", f);
        assert_node("name.erstername.zweitername", f);
    }

    #[test]
    fn element_name() {
        let f = |p: &mut Parser| p.parse_element_name().map(|_| Ok(()));
        assert_node("name", f);
        assert_node("*", f);
        assert_node("foo|h1", f);
        assert_node("foo|*", f);
        assert_node("|h1", f);
        assert_node("*|h1", f);
    }

    #[test]
    fn attrib() {
        let f = |p: &mut Parser| p.parse_attribute();
        assert_node("[name]", f);
        assert_node("[name = name2]", f);
        assert_node("[name ~= name3]", f);
        assert_node("[name~=name3]", f);
        assert_node("[name |= name3]", f);
        assert_node("[name |= \"this is a striiiing\"]", f);
        assert_node("[href*=\"insensitive\" i]", f);
        assert_node("[href*=\"sensitive\" S]", f);

        // Single namespace
        assert_node("[namespace|name]", f);
        assert_node("[name-space|name = name2]", f);
        assert_node("[name_space|name ~= name3]", f);
        assert_node("[name0spae|name~=name3]", f);
        assert_node("[NameSpace|name |= \"this is a striiiing\"]", f);
        assert_node("[name\\*space|name |= name3]", f);
        assert_node("[*|name]", f);
    }

    #[test]
    fn pseudo() {
        let f = |p: &mut Parser| p.parse_pseudo_opt();
        assert_node(":some", f);
        assert_node(":some(thing)", f);
        assert_node(":nth-child(12)", f);
        assert_node(":nth-child(1n)", f);
        assert_node(":nth-child(-n+3)", f);
        assert_node(":nth-child(2n+1)", f);
        assert_node(":nth-child(2n+1 of .foo)", f);
        assert_node(
            ":nth-child(2n+1 of .foo > bar, :not(*) ~ [other=\"value\"])",
            f
        );
        assert_node(":lang(it)", f);
        assert_node(":not(.class)", f);
        assert_node(":not(:disabled)", f);
        assert_node(":not(#foo)", f);
        assert_node("::slotted(*)", f); // #35076
        assert_node("::slotted(div:hover)", f); // #35076
        assert_node(":global(.output ::selection)", f); // #49010
        assert_node(":matches(:hover, :focus)", f); // #49010
        assert_node(":host([foo=bar][bar=foo])", f); // #49589
        assert_node(":has(> .test)", f); // #250
        assert_node(":has(~ .test)", f); // #250
        assert_node(":has(+ .test)", f); // #250
        assert_node(":has(~ div .test)", f); // #250
        assert_error("::", f, IdentifierExpected);
        assert_error(":: foo", f, IdentifierExpected);
        assert_error(":nth-child(1n of)", f, SelectorExpected);
    }

    #[test]
    fn declaration() {
        let f = |p: &mut Parser| p.parse_declaration_opt(None);

        assert_node("name : \"this is a string\" !important", f);
        assert_node("name : \"this is a string\"", f);
        assert_node("property:12", f);
        assert_node("-vendor-property: 12", f);
        assert_node("font-size: 12px", f);
        assert_node("color : #888 /4", f);
        assert_node(
            "filter : progid:DXImageTransform.Microsoft.Shadow(color=#000000,direction=45)",
            f
        );
        assert_node("filter : progid: DXImageTransform.\nMicrosoft.\nDropShadow(\noffx=2, offy=1, color=#000000)", f);
        assert_node("font-size: 12px", f);
        assert_node("*background: #f00 /* IE 7 and below */", f);
        assert_node("_background: #f60 /* IE 6 and below */", f);
        assert_node("background-image: linear-gradient(to right, silver, white 50px, white calc(100% - 50px), silver)", f);
        assert_node(
            "grid-template-columns: [first nav-start] 150px [main-start] 1fr [last]",
            f
        );
        assert_node(
            "grid-template-columns: repeat(4, 10px [col-start] 250px [col-end]) 10px",
            f
        );
        assert_node("grid-template-columns: [a] auto [b] minmax(min-content, 1fr) [b c d] repeat(2, [e] 40px)", f);
        assert_node("grid-template: [foo] 10px / [bar] 10px", f);
        assert_node(
            "grid-template: 'left1 footer footer' 1fr [end] / [ini] 1fr [info-start] 2fr 1fr [end]",
            f
        );
        assert_node("content: \"(\"counter(foo) \")\"", f);
        assert_node("content: 'Hello\\0A''world'", f);
    }

    #[test]
    fn term() {
        let f = |p: &mut Parser| p.parse_term().map(|_| Ok(()));

        assert_node("\"asdasd\"", f);
        assert_node("name", f);
        assert_node("#FFFFFF", f);
        assert_node("url(\"this is a url\")", f);
        assert_node("+324", f);
        assert_node("-45", f);
        assert_node("+45", f);
        assert_node("-45%", f);
        assert_node("-45mm", f);
        assert_node("-45em", f);
        assert_node("\"asdsa\"", f);
        assert_node("faa", f);
        assert_node("url(\"this is a striiiiing\")", f);
        assert_node("#FFFFFF", f);
        assert_node("name(asd)", f);
        assert_node("calc(50% + 20px)", f);
        assert_node("calc(50% + (100%/3 - 2*1em - 2*1px))", f);
        assert_node("U+002?-0199", f);
    }

    #[test]
    #[should_panic(expected = "no tree at all")]
    fn no_term0() {
        assert_no_node(
            "%('repetitions: %S file: %S', 1 + 2, \"directory/file.less\")",
            |p: &mut Parser| p.parse_term().map(|_| Ok(())),
        ); // less syntax
    }
    #[test]
    #[should_panic(expected = "no tree at all")]
    fn no_term1() {
        assert_no_node(
            "~\"ms:alwaysHasItsOwnSyntax.For.Stuff()\"",
            |p: &mut Parser| p.parse_term().map(|_| Ok(())),
        ); // less syntax
    }
    #[test]
    fn no_term2() {
        assert_no_node("U+002?-01??", |p: &mut Parser| p.parse_term().map(|_| Ok(())));
        assert_no_node("U+00?0;", |p: &mut Parser| p.parse_term().map(|_| Ok(())));
        assert_no_node("U+0XFF;", |p: &mut Parser| p.parse_term().map(|_| Ok(())));
    }

    #[test]
    fn function() {
        let f = |p: &mut Parser| p.parse_function_with_args_opt();
        assert_node("name( \"bla\" )", f);
        assert_node("name( name )", f);
        assert_node("name( -500mm )", f);
        assert_node("\u{060f}rf()", f);
        assert_node("über()", f);
        assert_node("let(--color)", f);
        assert_node("let(--color, somevalue)", f);
        assert_node("let(--variable1, --variable2)", f);
        assert_node("let(--variable1, let(--variable2))", f);
        assert_node("fun(value1, value2)", f);
        assert_node("fun(value1,)", f);
    }

    #[test]
    #[should_panic(expected = "no tree at all")]
    fn function_not0() {
        assert_no_node("über ()", |p: &mut Parser| p.parse_function_with_args_opt());
    }
    #[test]
    #[should_panic(expected = "no tree at all")]
    fn function_not1() {
        assert_no_node("%()", |p: &mut Parser| p.parse_function_with_args_opt());
    }
    #[test]
    #[should_panic(expected = "no tree at all")]
    fn function_not2() {
        assert_no_node("% ()", |p: &mut Parser| p.parse_function_with_args_opt());
    }

    #[test]
    fn test_token_prio() {
        let f = |p: &mut Parser| p.parse_prio_opt().map(|_| Ok(()));

        assert_node("!important", f);
        assert_node("!/*demo*/important", f);
        assert_node("! /*demo*/ important", f);
        assert_node("! /*dem o*/  important", f);
    }

    #[test]
    fn hexcolor() {
        let f = |p: &mut Parser| p.parse_hex_color_opt().map(|_| Ok(()));

        assert_node("#FFF", f);
        assert_node("#FFFF", f);
        assert_node("#FFFFFF", f);
        assert_node("#FFFFFFFF", f);
    }

    #[test]
    fn test_class() {
        let ele_f = |p: &mut Parser| p.parse_element_name().map(|_| Ok(()));
        let f = |p: &mut Parser| p.parse_class_opt();
        assert_node(".faa", f);
        assert_node("faa", ele_f);
        assert_node("*", ele_f);
        assert_node(".faa42", f);
    }

    #[test]
    fn prio() {
        assert_node("!important", |p: &mut Parser| p.parse_prio_opt().map(|_| Ok(())));
    }

    #[test]
    fn expr() {
        let f = |p: &mut Parser| p.parse_expr_opt(false).map(|_| Ok(()));

        assert_node("45,5px", f);
        assert_node(" 45 , 5px ", f);
        assert_node("5/6", f);
        assert_node("36mm, -webkit-calc(100%-10px)", f);
    }

    #[test]
    fn url() {        
        let f = |p: &mut Parser| p.parse_uri_literal_opt().map(|_| Ok(()));

        assert_node("url(//yourdomain/yourpath.png)", f);
        assert_node("url('http://msft.com')", f);
        assert_node("url(\"http://msft.com\")", f);
        assert_node("url( \"http://msft.com\")", f);
        assert_node("url(\t\"http://msft.com\")", f);
        assert_node("url(\n\"http://msft.com\")", f);
        assert_node("url(\"http://msft.com\"\n)", f);
        assert_node("url(\"\")", f);
        assert_node("uRL(\"\")", f);
        assert_node("URL(\"\")", f);
        assert_node("url(http://msft.com)", f);
        assert_node("url()", f);
        assert_node("url('http://msft.com\n)", f);
        assert_error(
            "url(\"http://msft.com\"",
            f,
            RightParenthesisExpected
        );
        assert_node("url(http://msft.com')", f); // parsed bad url
    }
}
