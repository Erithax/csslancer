
pub mod syntax_kind_src;
pub mod astgen;
pub mod syntax_kind_gen;
pub mod syntax_kind_ext;
pub mod nodes_gen;
pub mod tokens_gen;
pub mod nodes_types;
pub mod syntax_error;
pub mod ast;
pub mod lex_to_syn;
pub mod shortcut;
pub mod input;
pub mod output;
pub mod parser;
pub mod css_grammar;
pub mod css_grammar_test;
pub mod event;
pub mod token_set;
pub mod parse_error;
pub mod reparsing;
pub mod fuzz;

use std::marker::PhantomData;

use itertools::Itertools;
//use stdx::format_to;
use ra_ap_text_edit::Indel;
use triomphe::Arc;

use self::{
    ast::AstNode,
    //ptr::{AstPtr, SyntaxNodePtr},
    syntax_error::SyntaxError,
    nodes_types::{
        SyntaxNode, SyntaxTreeBuilder,
    },
    //token_text::TokenText,
};
//use parser::{SyntaxKind, T};
use syntax_kind_gen::SyntaxKind;
use crate::T;
use rowan::{
    GreenNode, TextRange, TextSize,
};

use nodes_gen::SourceFile;

// #[derive(Debug)]
// pub struct Parse {
//     pub no_dump: bool,
// }

/// `Parse` is the result of the parsing: a syntax tree and a collection of
/// errors.
///
/// Note that we always produce a syntax tree, even for completely invalid
/// files.
#[derive(Debug, PartialEq, Eq)]
pub struct Parse<T> {
    green: GreenNode,
    errors: Option<Arc<[SyntaxError]>>,
    _ty: PhantomData<fn() -> T>,
}

impl<T> Clone for Parse<T> {
    fn clone(&self) -> Parse<T> {
        Parse { green: self.green.clone(), errors: self.errors.clone(), _ty: PhantomData }
    }
}

impl<T> Parse<T> {
    fn new(green: GreenNode, errors: Vec<SyntaxError>) -> Parse<T> {
        Parse {
            green,
            errors: if errors.is_empty() { None } else { Some(errors.into()) },
            _ty: PhantomData,
        }
    }

    pub fn syntax_node(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }

    pub fn errors(&self) -> Vec<SyntaxError> {
        let errors = if let Some(e) = self.errors.as_deref() { e.to_vec() } else { vec![] };
        // TODO validation::validate(&self.syntax_node(), &mut errors);
        errors
    }

    pub fn fancy_string(&self) -> String {
        let mut res = String::new();
        res += "Parse";
        res += "\n    Errors: ";
        res += &self.errors
            .as_ref()
            .map(|errs| 
                "\n        ".to_string() + &(*errs).iter()
                    .map(|e| format!("{} at {:?}", e, e.range()))
                    .join(";\n        ")
            ).unwrap_or_default();
        res += &Self::fancy_string_internal(&self.syntax_node(), 0);
        res
    }

    fn fancy_string_internal(syntax_node: &SyntaxNode, ident: usize) -> String {
        let ident_s = "    ".repeat(ident);
        "\n".to_owned()
            + &ident_s
            + &format!(
                "{:?}[{:?}]({:?}+{:?}={:?}) {{",
                syntax_node.kind(),
                syntax_node.index(),
                syntax_node.text_range().start(),
                syntax_node.text_range().len(),
                syntax_node.text_range().end(),
            )
            + &syntax_node
                .children()
                .map(|ch| Self::fancy_string_internal(&ch, ident + 1))
                .fold(String::new(), |acc, nex| acc + &nex)
            + "\n"
            + &ident_s
            + "}"
    }
}

impl<T: AstNode> Parse<T> {
    pub fn to_syntax(self) -> Parse<SyntaxNode> {
        Parse { green: self.green, errors: self.errors, _ty: PhantomData }
    }

    pub fn tree(&self) -> T {
        T::cast(self.syntax_node()).unwrap()
    }

    pub fn ok(self) -> Result<T, Vec<SyntaxError>> {
        match self.errors() {
            errors if !errors.is_empty() => Err(errors),
            _ => Ok(self.tree()),
        }
    }
}

impl Parse<SyntaxNode> {
    pub fn cast<N: AstNode>(self) -> Option<Parse<N>> {
        if N::cast(self.syntax_node()).is_some() {
            Some(Parse { green: self.green, errors: self.errors, _ty: PhantomData })
        } else {
            None
        }
    }
}

impl Parse<SourceFile> {
    pub fn debug_dump(&self) -> String {
        let mut buf = format!("{:#?}", self.tree().syntax());
        for err in self.errors() {
            buf += &format!("error {:?}: {}\n", err.range(), err);
        }
        buf
    }

    pub fn reparse(&self, indel: &Indel) -> Parse<SourceFile> {
        self.incremental_reparse(indel).unwrap_or_else(|| self.full_reparse(indel))
    }

    fn incremental_reparse(&self, indel: &Indel) -> Option<Parse<SourceFile>> {
        // FIXME: validation errors are not handled here
        reparsing::incremental_reparse(
            self.tree().syntax(),
            indel,
            self.errors.as_deref().unwrap_or_default().iter().cloned(),
        )
        .map(|(green_node, errors, _reparsed_range)| Parse {
            green: green_node,
            errors: if errors.is_empty() { None } else { Some(errors.into()) },
            _ty: PhantomData,
        })
    }

    fn full_reparse(&self, indel: &Indel) -> Parse<SourceFile> {
        let mut text = self.tree().syntax().text().to_string();
        indel.apply(&mut text);
        SourceFile::parse(&text)
    }
}



// Lexing, bridging to parser (which does the actual parsing) and
// incremental reparsing.

// use rowan::GreenNode;
// use nodes_types::SyntaxTreeBuilder;
// use syntax_error::SyntaxError;

// pub(crate) use crate::parsing::reparsing::incremental_reparse;


pub(crate) fn must_parse_fn<F: Fn(&mut parser::Parser) -> Option<()>>(input: &input::Input, f: F) -> (bool, output::Output) {
    //let _p = tracing::span!(tracing::Level::INFO, "TopEntryPoint::parse", ?self).entered();
    let mut p = parser::Parser::new(input);
    let success = f(&mut p).is_some();
    let events = p.finish();
    let res = event::process(events);

    if cfg!(debug_assertions) {
        let mut depth = 0;
        let mut first = true;
        for step in res.iter() {
            assert!(depth > 0 || first, "depth = {}; first = {}", depth, first);
            first = false;
            match step {
                output::Step::Enter { .. } => depth += 1,
                output::Step::Exit => depth -= 1,
                output::Step::FloatSplit { ends_in_dot: has_pseudo_dot } => {
                    depth -= 1 + !has_pseudo_dot as usize
                }
                output::Step::Token { .. } | output::Step::Error { .. } => (),
            }
        }
        assert!(!first, "no tree at all");
        assert_eq!(depth, 0, "unbalanced tree");
    }

    (success, res)
}

pub(crate) fn build_tree(
    lexed: lex_to_syn::LexedStr<'_>,
    parser_output: output::Output,
) -> (GreenNode, Vec<SyntaxError>, bool) {
    let _p = tracing::span!(tracing::Level::INFO, "build_tree").entered();
    let mut builder = SyntaxTreeBuilder::default();

    let is_eof = lexed.intersperse_trivia(&parser_output, &mut |step| match step {
        shortcut::StrStep::Token { kind, text } => builder.token(kind, text),
        shortcut::StrStep::Enter { kind } => builder.start_node(kind),
        shortcut::StrStep::Exit => builder.finish_node(),
        shortcut::StrStep::Error { msg, pos } => {
            builder.error(msg.to_owned(), pos.try_into().unwrap())
        }
    });

    let (node, mut errors) = builder.finish_raw();
    for (i, err) in lexed.errors() {
        let text_range = lexed.text_range(i);
        let text_range = TextRange::new(
            text_range.start.try_into().unwrap(),
            text_range.end.try_into().unwrap(),
        );
        errors.push(SyntaxError::new(err, text_range))
    }

    (node, errors, is_eof)
}

pub(crate) fn must_parse_text_as_fn<F: Fn(&mut parser::Parser) -> Option<()>>(text: &str, f: F) -> (bool, (GreenNode, Vec<SyntaxError>)) {
    let _p = tracing::span!(tracing::Level::INFO, "must_parse_text_as_fn").entered();
    let lexed = lex_to_syn::LexedStr::new(text);
    let parser_input = lexed.to_input();
    let (success, parser_output) = must_parse_fn(&parser_input, f);
    let (node, errors, _eof) = build_tree(lexed, parser_output);
    (success, (node, errors))
}

#[inline]
pub(crate) fn parse_text_as_fn<F: Fn(&mut parser::Parser) -> Option<()>>(text: &str, f: F) -> (GreenNode, Vec<SyntaxError>) {
    must_parse_text_as_fn(text, f).1
}

#[inline]
pub(crate) fn parse_source_file_text(text: &str) -> (GreenNode, Vec<SyntaxError>) {
    parse_text_as_fn(text, |p: &mut parser::Parser| {p.parse_source_file(); Some(())})
}

impl SourceFile {
    pub fn parse(text: &str) -> Parse<SourceFile> {
        let _p = tracing::span!(tracing::Level::INFO, "SourceFile::parse").entered();
        let (green, errors) = parse_source_file_text(text);
        let root = SyntaxNode::new_root(green.clone());

        assert_eq!(root.kind(), SyntaxKind::SOURCE_FILE);
        Parse {
            green,
            errors: if errors.is_empty() { None } else { Some(errors.into()) },
            _ty: PhantomData,
        }
    }
}


// impl ast::TokenTree {
//     pub fn reparse_as_comma_separated_expr(self) -> Parse<ast::MacroEagerInput> {
//         let tokens = self.syntax().descendants_with_tokens().filter_map(NodeOrToken::into_token);

//         let mut parser_input = parser::Input::default();
//         let mut was_joint = false;
//         for t in tokens {
//             let kind = t.kind();
//             if kind.is_trivia() {
//                 was_joint = false
//             } else if kind == SyntaxKind::IDENT {
//                 let token_text = t.text();
//                 let contextual_kw =
//                     SyntaxKind::from_contextual_keyword(token_text).unwrap_or(SyntaxKind::IDENT);
//                 parser_input.push_ident(contextual_kw);
//             } else {
//                 if was_joint {
//                     parser_input.was_joint();
//                 }
//                 parser_input.push(kind);
//                 // Tag the token as joint if it is float with a fractional part
//                 // we use this jointness to inform the parser about what token split
//                 // event to emit when we encounter a float literal in a field access
//                 if kind == SyntaxKind::FLOAT_NUMBER {
//                     if !t.text().ends_with('.') {
//                         parser_input.was_joint();
//                     } else {
//                         was_joint = false;
//                     }
//                 } else {
//                     was_joint = true;
//                 }
//             }
//         }

//         let parser_output = parser::TopEntryPoint::MacroEagerInput.parse(&parser_input);

//         let mut tokens =
//             self.syntax().descendants_with_tokens().filter_map(NodeOrToken::into_token);
//         let mut text = String::new();
//         let mut pos = TextSize::from(0);
//         let mut builder = SyntaxTreeBuilder::default();
//         for event in parser_output.iter() {
//             match event {
//                 parser::Step::Token { kind, n_input_tokens } => {
//                     let mut token = tokens.next().unwrap();
//                     while token.kind().is_trivia() {
//                         let text = token.text();
//                         pos += TextSize::from(text.len() as u32);
//                         builder.token(token.kind(), text);

//                         token = tokens.next().unwrap();
//                     }
//                     text.push_str(token.text());
//                     for _ in 1..n_input_tokens {
//                         let token = tokens.next().unwrap();
//                         text.push_str(token.text());
//                     }

//                     pos += TextSize::from(text.len() as u32);
//                     builder.token(kind, &text);
//                     text.clear();
//                 }
//                 parser::Step::FloatSplit { ends_in_dot: has_pseudo_dot } => {
//                     let token = tokens.next().unwrap();
//                     let text = token.text();

//                     match text.split_once('.') {
//                         Some((left, right)) => {
//                             assert!(!left.is_empty());
//                             builder.start_node(SyntaxKind::NAME_REF);
//                             builder.token(SyntaxKind::INT_NUMBER, left);
//                             builder.finish_node();

//                             // here we move the exit up, the original exit has been deleted in process
//                             builder.finish_node();

//                             builder.token(SyntaxKind::DOT, ".");

//                             if has_pseudo_dot {
//                                 assert!(right.is_empty(), "{left}.{right}");
//                             } else {
//                                 assert!(!right.is_empty(), "{left}.{right}");
//                                 builder.start_node(SyntaxKind::NAME_REF);
//                                 builder.token(SyntaxKind::INT_NUMBER, right);
//                                 builder.finish_node();

//                                 // the parser creates an unbalanced start node, we are required to close it here
//                                 builder.finish_node();
//                             }
//                         }
//                         None => unreachable!(),
//                     }
//                     pos += TextSize::from(text.len() as u32);
//                 }
//                 parser::Step::Enter { kind } => builder.start_node(kind),
//                 parser::Step::Exit => builder.finish_node(),
//                 parser::Step::Error { msg } => builder.error(msg.to_owned(), pos),
//             }
//         }

//         let (green, errors) = builder.finish_raw();

//         Parse {
//             green,
//             errors: if errors.is_empty() { None } else { Some(errors.into()) },
//             _ty: PhantomData,
//         }
//     }
// }

/// Matches a `SyntaxNode` against an `ast` type.
///
/// # Example:
///
/// ```ignore
/// match_ast! {
///     match node {
///         ast::CallExpr(it) => { ... },
///         ast::MethodCallExpr(it) => { ... },
///         ast::MacroCall(it) => { ... },
///         _ => None,
///     }
/// }
/// ```
#[macro_export]
macro_rules! match_ast {
    (match $node:ident { $($tt:tt)* }) => { $crate::match_ast!(match ($node) { $($tt)* }) };

    (match ($node:expr) {
        $( $( $path:ident )::+ ($it:pat) => $res:expr, )*
        _ => $catch_all:expr $(,)?
    }) => {{
        $( if let Some($it) = $($path::)+cast($node.clone()) { $res } else )*
        { $catch_all }
    }};
}

// This test does not assert anything and instead just shows off the crate's
// API.
// #[test]
// fn api_walkthrough() {
//     //use ast::{HasModuleItem, HasName};

//     let source_code = "
//         fn foo() {
//             1 + 1
//         }
//     ";
//     // `SourceFile` is the main entry point.
//     //
//     // The `parse` method returns a `Parse` -- a pair of syntax tree and a list
//     // of errors. That is, syntax tree is constructed even in presence of errors.
//     let parse = SourceFile::parse(source_code);
//     assert!(parse.errors().is_empty());

//     // The `tree` method returns an owned syntax node of type `SourceFile`.
//     // Owned nodes are cheap: inside, they are `Rc` handles to the underling data.
//     let file: SourceFile = parse.tree();

//     // `SourceFile` is the root of the syntax tree. We can iterate file's items.
//     // Let's fetch the `foo` function.
//     let mut func = None;
//     for item in file.items() {
//         match item {
//             ast::Item::Fn(f) => func = Some(f),
//             _ => unreachable!(),
//         }
//     }
//     let func: ast::Fn = func.unwrap();

//     // Each AST node has a bunch of getters for children. All getters return
//     // `Option`s though, to account for incomplete code. Some getters are common
//     // for several kinds of node. In this case, a trait like `ast::NameOwner`
//     // usually exists. By convention, all ast types should be used with `ast::`
//     // qualifier.
//     let name: Option<ast::Name> = func.name();
//     let name = name.unwrap();
//     assert_eq!(name.text(), "foo");

//     // Let's get the `1 + 1` expression!
//     let body: ast::BlockExpr = func.body().unwrap();
//     let stmt_list: ast::StmtList = body.stmt_list().unwrap();
//     let expr: ast::Expr = stmt_list.tail_expr().unwrap();

//     // Enums are used to group related ast nodes together, and can be used for
//     // matching. However, because there are no public fields, it's possible to
//     // match only the top level enum: that is the price we pay for increased API
//     // flexibility
//     let bin_expr: &ast::BinExpr = match &expr {
//         ast::Expr::BinExpr(e) => e,
//         _ => unreachable!(),
//     };

//     // Besides the "typed" AST API, there's an untyped CST one as well.
//     // To switch from AST to CST, call `.syntax()` method:
//     let expr_syntax: &SyntaxNode = expr.syntax();

//     // Note how `expr` and `bin_expr` are in fact the same node underneath:
//     assert!(expr_syntax == bin_expr.syntax());

//     // To go from CST to AST, `AstNode::cast` function is used:
//     let _expr: ast::Expr = match ast::Expr::cast(expr_syntax.clone()) {
//         Some(e) => e,
//         None => unreachable!(),
//     };

//     // The two properties each syntax node has is a `SyntaxKind`:
//     assert_eq!(expr_syntax.kind(), SyntaxKind::BIN_EXPR);

//     // And text range:
//     assert_eq!(expr_syntax.text_range(), TextRange::new(32.into(), 37.into()));

//     // You can get node's text as a `SyntaxText` object, which will traverse the
//     // tree collecting token's text:
//     let text: SyntaxText = expr_syntax.text();
//     assert_eq!(text.to_string(), "1 + 1");

//     // There's a bunch of traversal methods on `SyntaxNode`:
//     assert_eq!(expr_syntax.parent().as_ref(), Some(stmt_list.syntax()));
//     assert_eq!(stmt_list.syntax().first_child_or_token().map(|it| it.kind()), Some(T!['{']));
//     assert_eq!(
//         expr_syntax.next_sibling_or_token().map(|it| it.kind()),
//         Some(SyntaxKind::WHITESPACE)
//     );

//     // As well as some iterator helpers:
//     let f = expr_syntax.ancestors().find_map(ast::Fn::cast);
//     assert_eq!(f, Some(func));
//     assert!(expr_syntax.siblings_with_tokens(Direction::Next).any(|it| it.kind() == T!['}']));
//     assert_eq!(
//         expr_syntax.descendants_with_tokens().count(),
//         8, // 5 tokens `1`, ` `, `+`, ` `, `!`
//            // 2 child literal expressions: `1`, `1`
//            // 1 the node itself: `1 + 1`
//     );

//     // There's also a `preorder` method with a more fine-grained iteration control:
//     let mut buf = String::new();
//     let mut indent = 0;
//     for event in expr_syntax.preorder_with_tokens() {
//         match event {
//             WalkEvent::Enter(node) => {
//                 let text = match &node {
//                     NodeOrToken::Node(it) => it.text().to_string(),
//                     NodeOrToken::Token(it) => it.text().to_owned(),
//                 };
//                 format_to!(buf, "{:indent$}{:?} {:?}\n", " ", text, node.kind(), indent = indent);
//                 indent += 2;
//             }
//             WalkEvent::Leave(_) => indent -= 2,
//         }
//     }
//     assert_eq!(indent, 0);
//     assert_eq!(
//         buf.trim(),
//         r#"
// "1 + 1" BIN_EXPR
//   "1" LITERAL
//     "1" INT_NUMBER
//   " " WHITESPACE
//   "+" PLUS
//   " " WHITESPACE
//   "1" LITERAL
//     "1" INT_NUMBER
// "#
//         .trim()
//     );

//     // To recursively process the tree, there are three approaches:
//     // 1. explicitly call getter methods on AST nodes.
//     // 2. use descendants and `AstNode::cast`.
//     // 3. use descendants and `match_ast!`.
//     //
//     // Here's how the first one looks like:
//     let exprs_cast: Vec<String> = file
//         .syntax()
//         .descendants()
//         .filter_map(ast::Expr::cast)
//         .map(|expr| expr.syntax().text().to_string())
//         .collect();

//     // An alternative is to use a macro.
//     let mut exprs_visit = Vec::new();
//     for node in file.syntax().descendants() {
//         match_ast! {
//             match node {
//                 ast::Expr(it) => {
//                     let res = it.syntax().text().to_string();
//                     exprs_visit.push(res);
//                 },
//                 _ => (),
//             }
//         }
//     }
//     assert_eq!(exprs_cast, exprs_visit);
// }
