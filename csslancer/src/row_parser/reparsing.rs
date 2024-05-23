//! Implementation of incremental re-parsing.
//!
//! We use two simple strategies for this:
//!   - if the edit modifies only a single token (like changing an identifier's
//!     letter), we replace only this token.
//!   - otherwise, we search for the nearest `{}` block which contains the edit
//!     and try to parse only this block.

use ra_ap_text_edit::Indel;

use super::{
    build_tree, 
    event, 
    input::Input, 
    lex_to_syn, 
    nodes_types::{SyntaxNode, SyntaxElement},
    output::Output, parser::Parser, syntax_kind_gen::SyntaxKind, SyntaxError, SyntaxKind::*, TextRange, TextSize, T
};

use rowan::{GreenNode, GreenToken, NodeOrToken};

pub(crate) fn reparser(node: &SyntaxNode) -> Option<fn(&mut Parser<'_>) -> Option<SyntaxKind>> {
    if node.kind() != SyntaxKind::DECLARATIONS {
        return None
    }
    let parent = node.parent()?;

    let is_nested = node.ancestors().any(|a| a.kind() == SyntaxKind::DECLARATIONS);

    // let optionally_nested_fn = |is_nested: bool, f: fn(&mut Parser<'_>, bool)| -> fn(&mut Parser) {
    //     if is_nested {
    //         return |p: &mut Parser| f(p, true)
    //     } else {
    //         return |p: &mut Parser| f(p, false)
    //     }
    // };


    let res = match parent.kind() {
        SyntaxKind::RULE_SET => |p: &mut Parser| p.parse_rule_set_declaration_opt(),
        SyntaxKind::FONT_FACE => |p: &mut Parser| p.parse_rule_set_declaration_opt(),
        SyntaxKind::VIEW_PORT => |p: &mut Parser| p.parse_rule_set_declaration_opt(),
        SyntaxKind::KEYFRAME => |p: &mut Parser| p.parse_keyframe_selector_opt().map(|_| SyntaxKind::KEYFRAME_SELECTOR),
        SyntaxKind::KEYFRAME_SELECTOR => |p: &mut Parser| p.parse_rule_set_declaration_opt(),
        SyntaxKind::PROPERTY_AT_RULE => |p: &mut Parser| p.parse_declaration_opt(None).map(|_| SyntaxKind::DECLARATION),
        SyntaxKind::LAYER => if is_nested {
            |p: &mut Parser| p.parse_layer_declaration(true)
        } else {
            |p: &mut Parser| p.parse_layer_declaration(false)
        },
        SyntaxKind::SUPPORTS => if is_nested {
            |p: &mut Parser| p.parse_supports_declaration(true)
        } else {
            |p: &mut Parser| p.parse_supports_declaration(false)
        },
        SyntaxKind::MEDIA => if is_nested {
            |p: &mut Parser| p.parse_media_declaration(true)
        } else {
            |p: &mut Parser| p.parse_media_declaration(false)
        },
        SyntaxKind::PAGE => |p: &mut Parser| p.parse_page_declaration(),
        SyntaxKind::PAGE_BOX_MARGIN_BOX => |p: &mut Parser| p.parse_rule_set_declaration_opt(),
        SyntaxKind::DOCUMENT => if is_nested {
            |p: &mut Parser| p.parse_stylesheet_statement_opt(true)
        } else {
            |p: &mut Parser| p.parse_stylesheet_statement_opt(false)
        },
        SyntaxKind::CONTAINER => if is_nested {
            |p: &mut Parser| p.parse_stylesheet_statement_opt(true)
        } else {
            |p: &mut Parser| p.parse_stylesheet_statement_opt(false)
        },
        _ => return None
    };

    Some(res)

    // let res = match node {
    //     BLOCK_EXPR => expressions::block_expr,
    //     RECORD_FIELD_LIST => items::record_field_list,
    //     RECORD_EXPR_FIELD_LIST => items::record_expr_field_list,
    //     VARIANT_LIST => items::variant_list,
    //     MATCH_ARM_LIST => items::match_arm_list,
    //     USE_TREE_LIST => items::use_tree_list,
    //     EXTERN_ITEM_LIST => items::extern_item_list,
    //     TOKEN_TREE if first_child? == T!['{'] => items::token_tree,
    //     ASSOC_ITEM_LIST => match parent? {
    //         IMPL | TRAIT => items::assoc_item_list,
    //         _ => return None,
    //     },
    //     ITEM_LIST => items::item_list,
    //     _ => return None,
    // };
    // Some(res)
}


/// A parsing function for a specific braced-block.
pub struct DeclarationsReparser(fn(&mut Parser<'_>) -> Option<SyntaxKind>);

impl DeclarationsReparser {
    /// If the node is a braced block, return the corresponding `Reparser`.
    #[inline]
    pub fn for_node(
        node: &SyntaxNode
    ) -> Option<DeclarationsReparser> {
        reparser(node).map(DeclarationsReparser)
    }

    /// Re-parse given tokens using this `DeclarationsReparser`.
    ///
    /// Tokens must start with `{`, end with `}` and form a valid brace
    /// sequence.
    pub fn parse(self, tokens: &Input) -> Option<Output> {
        debug_assert_eq!(SyntaxKind::L_CURLY, tokens.kind(0));
        debug_assert_eq!(SyntaxKind::R_CURLY, tokens.last_kind());
        let DeclarationsReparser(r) = self;
        let mut p = Parser::new(tokens);
        p.parse_body(r);
        if !p.at(SyntaxKind::EOF) {
            return None
        }
        let events = p.finish();
        Some(event::process(events))
    }
}

pub(crate) fn incremental_reparse(
    node: &SyntaxNode,
    edit: &Indel,
    errors: impl IntoIterator<Item = SyntaxError>,
) -> Option<(GreenNode, Vec<SyntaxError>, TextRange)> {
    if let Some((green, new_errors, old_range)) = reparse_token(node, edit) {
        return Some((green, merge_errors(errors, new_errors, old_range, edit), old_range));
    }

    if let Some((green, new_errors, old_range)) =
        reparse_block(node, edit)
    {
        return Some((green, merge_errors(errors, new_errors, old_range, edit), old_range));
    }
    None
}

fn reparse_token(
    root: &SyntaxNode,
    edit: &Indel,
) -> Option<(GreenNode, Vec<SyntaxError>, TextRange)> {
    assert!(
        root.text_range().contains_range(edit.delete),
        "Bad range: node range {:?}, range {:?}",
        root.text_range(),
        edit.delete,
    );
    let prev_token = root.covering_element(edit.delete).as_token()?.clone();
    let prev_token_kind = prev_token.kind();
    match prev_token_kind {
        SyntaxKind::WHITESPACE | SyntaxKind::COMMENT | SyntaxKind::IDENTIFIER | SyntaxKind::STRING | SyntaxKind::BAD_STRING | SyntaxKind::URL | SyntaxKind::BAD_URL => {
            if prev_token_kind == WHITESPACE || prev_token_kind == COMMENT {
                // removing a new line may extends previous token
                let deleted_range = edit.delete - prev_token.text_range().start();
                if prev_token.text()[deleted_range].contains('\n') {
                    return None;
                }
            }

            let mut new_text = get_text_after_edit(prev_token.clone().into(), edit);
            let (new_token_kind, new_err) = lex_to_syn::LexedStr::single_token(&new_text)?;

            if new_token_kind != prev_token_kind
                || (new_token_kind == SyntaxKind::IDENTIFIER && is_contextual_kw(&new_text))
            {
                return None;
            }

            // Check that edited token is not a part of the bigger token.
            // E.g. if for source code `bruh"str"` the user removed `ruh`, then
            // `b` no longer remains an identifier, but becomes a part of byte string literal
            if let Some(next_char) = root.text().char_at(prev_token.text_range().end()) {
                new_text.push(next_char);
                let token_with_next_char = lex_to_syn::LexedStr::single_token(&new_text);
                if let Some((_kind, _error)) = token_with_next_char {
                    return None;
                }
                new_text.pop();
            }

            let new_token = GreenToken::new(rowan::SyntaxKind(prev_token_kind.into()), &new_text);
            let range = TextRange::up_to(TextSize::of(&new_text));
            Some((
                prev_token.replace_with(new_token),
                new_err.into_iter().map(|msg| SyntaxError::new(msg, range)).collect(),
                prev_token.text_range(),
            ))
        }
        _ => None,
    }
}

fn reparse_block(
    root: &SyntaxNode,
    edit: &Indel,
) -> Option<(GreenNode, Vec<SyntaxError>, TextRange)> {
    let (child_declarations_node, reparser) = find_reparsable_node(root, edit.delete)?;
    assert_eq!(SyntaxKind::DECLARATIONS, child_declarations_node.kind());
    //let node = child_declarations_node.parent().unwrap();
    let text = get_text_after_edit(child_declarations_node.clone().into(), edit);
    let node = child_declarations_node;

    let lexed = lex_to_syn::LexedStr::new(text.as_str());
    let parser_input = lexed.to_input();
    if !is_braced_n_balanced(&lexed) {
        return None;
    }

    let tree_traversal = reparser.parse(&parser_input)?;

    let (green, new_parser_errors, _eof) = build_tree(lexed, tree_traversal);
    assert_eq!(<SyntaxKind as Into<u16>>::into(node.kind()), green.kind().0);
    Some((node.replace_with(green), new_parser_errors, node.text_range()))
}

fn get_text_after_edit(element: SyntaxElement, edit: &Indel) -> String {
    let edit = Indel::replace(edit.delete - element.text_range().start(), edit.insert.clone());

    let mut text = match element {
        NodeOrToken::Token(ref token) => token.text().to_owned(),
        NodeOrToken::Node(ref node) => node.text().to_string(),
    };
    edit.apply(&mut text);
    debug_assert_eq!(text, {
        let mut s = match element {NodeOrToken::Token(token) => token.text().to_owned(), NodeOrToken::Node(node) => node.text().to_string()};
        s.replace_range((<TextSize as Into<usize>>::into(edit.delete.start()))..(<TextSize as Into<usize>>::into(edit.delete.end())), &edit.insert);
        s
    });
    text
}

fn is_contextual_kw(text: &str) -> bool {
    matches!(text, "auto" | "default" | "union")
}

fn find_reparsable_node(node: &SyntaxNode, range: TextRange) -> Option<(SyntaxNode, DeclarationsReparser)> {
    assert!(
        node.text_range().contains_range(range),
        "Bad range: node range {:?}, range {:?}",
        node.text_range(),
        range,
    );
    let node = node.covering_element(range);

    node.ancestors().find_map(|node| {
        // let first_child = node.first_child_or_token().map(|it| it.kind());
        // let parent = node.parent().map(|it| it.kind());
        // Reparser::for_node(node.kind(), first_child, parent).map(|r| (node, r))
        DeclarationsReparser::for_node(&node).map(|r| (node, r))
    })
}

fn is_braced_n_balanced(lexed: &lex_to_syn::LexedStr<'_>) -> bool {
    if lexed.is_empty() || lexed.kind(0) != T!['{'] || lexed.kind(lexed.len() - 1) != T!['}'] {
        return false;
    }
    let mut balance = 0usize;
    for i in 1..lexed.len() - 1 {
        match lexed.kind(i) {
            T!['{'] => balance += 1,
            T!['}'] => {
                balance = match balance.checked_sub(1) {
                    Some(b) => b,
                    None => return false,
                }
            }
            _ => (),
        }
    }
    balance == 0
}

fn merge_errors(
    old_errors: impl IntoIterator<Item = SyntaxError>,
    new_errors: Vec<SyntaxError>,
    range_before_reparse: TextRange,
    edit: &Indel,
) -> Vec<SyntaxError> {
    let mut res = Vec::new();

    for old_err in old_errors {
        let old_err_range = old_err.range();
        if old_err_range.end() <= range_before_reparse.start() {
            res.push(old_err);
        } else if old_err_range.start() >= range_before_reparse.end() {
            let inserted_len = TextSize::of(&edit.insert);
            res.push(old_err.with_range((old_err_range + inserted_len) - edit.delete.len()));
            // Note: extra parens are intentional to prevent uint underflow, HWAB (here was a bug)
        }
    }
    res.extend(new_errors.into_iter().map(|new_err| {
        // fighting borrow checker with a variable ;)
        let offsetted_range = new_err.range() + range_before_reparse.start();
        new_err.with_range(offsetted_range)
    }));
    res.dedup_by_key(|e| format!("{:?}{:?}{}", e.range().start(), e.range().end(), e.to_string()));
    res
}

// #[cfg(test)]
// mod tests {
//     //use test_utils::{assert_eq_text, extract_range};

//     use super::*;
//     use super::super::{AstNode, Parse, SourceFile};

//     fn do_check(before: &str, replace_with: &str, reparsed_len: u32) {
//         let (range, before) = extract_range(before);
//         let edit = Indel::replace(range, replace_with.to_owned());
//         let after = {
//             let mut after = before.clone();
//             edit.apply(&mut after);
//             after
//         };

//         let fully_reparsed = SourceFile::parse(&after);
//         let incrementally_reparsed: Parse<SourceFile> = {
//             let before = SourceFile::parse(&before);
//             let (green, new_errors, range) = incremental_reparse(
//                 before.tree().syntax(),
//                 &edit,
//                 before.errors.as_deref().unwrap_or_default().iter().cloned(),
//             )
//             .unwrap();
//             assert_eq!(range.len(), reparsed_len.into(), "reparsed fragment has wrong length");
//             Parse::new(green, new_errors)
//         };

//         assert_eq_text!(
//             &format!("{:#?}", fully_reparsed.tree().syntax()),
//             &format!("{:#?}", incrementally_reparsed.tree().syntax()),
//         );
//         assert_eq!(fully_reparsed.errors(), incrementally_reparsed.errors());
//     }

//     #[test] // FIXME: some test here actually test token reparsing
//     fn reparse_block_tests() {
//         do_check(
//             r"
// fn foo() {
//     let x = foo + $0bar$0
// }
// ",
//             "baz",
//             3,
//         );
//         do_check(
//             r"
// fn foo() {
//     let x = foo$0 + bar$0
// }
// ",
//             "baz",
//             25,
//         );
//         do_check(
//             r"
// struct Foo {
//     f: foo$0$0
// }
// ",
//             ",\n    g: (),",
//             14,
//         );
//         do_check(
//             r"
// fn foo {
//     let;
//     1 + 1;
//     $092$0;
// }
// ",
//             "62",
//             31, // FIXME: reparse only int literal here
//         );
//         do_check(
//             r"
// mod foo {
//     fn $0$0
// }
// ",
//             "bar",
//             11,
//         );

//         do_check(
//             r"
// trait Foo {
//     type $0Foo$0;
// }
// ",
//             "Output",
//             3,
//         );
//         do_check(
//             r"
// impl IntoIterator<Item=i32> for Foo {
//     f$0$0
// }
// ",
//             "n next(",
//             9,
//         );
//         do_check(r"use a::b::{foo,$0,bar$0};", "baz", 10);
//         do_check(
//             r"
// pub enum A {
//     Foo$0$0
// }
// ",
//             "\nBar;\n",
//             11,
//         );
//         do_check(
//             r"
// foo!{a, b$0$0 d}
// ",
//             ", c[3]",
//             8,
//         );
//         do_check(
//             r"
// fn foo() {
//     vec![$0$0]
// }
// ",
//             "123",
//             14,
//         );
//         do_check(
//             r"
// extern {
//     fn$0;$0
// }
// ",
//             " exit(code: c_int)",
//             11,
//         );
//     }

//     #[test]
//     fn reparse_token_tests() {
//         do_check(
//             r"$0$0
// fn foo() -> i32 { 1 }
// ",
//             "\n\n\n   \n",
//             1,
//         );
//         do_check(
//             r"
// fn foo() -> $0$0 {}
// ",
//             "  \n",
//             2,
//         );
//         do_check(
//             r"
// fn $0foo$0() -> i32 { 1 }
// ",
//             "bar",
//             3,
//         );
//         do_check(
//             r"
// fn foo$0$0foo() {  }
// ",
//             "bar",
//             6,
//         );
//         do_check(
//             r"
// fn foo /* $0$0 */ () {}
// ",
//             "some comment",
//             6,
//         );
//         do_check(
//             r"
// fn baz $0$0 () {}
// ",
//             "    \t\t\n\n",
//             2,
//         );
//         do_check(
//             r"
// fn baz $0$0 () {}
// ",
//             "    \t\t\n\n",
//             2,
//         );
//         do_check(
//             r"
// /// foo $0$0omment
// mod { }
// ",
//             "c",
//             14,
//         );
//         do_check(
//             r#"
// fn -> &str { "Hello$0$0" }
// "#,
//             ", world",
//             7,
//         );
//         do_check(
//             r#"
// fn -> &str { // "Hello$0$0"
// "#,
//             ", world",
//             10,
//         );
//         do_check(
//             r##"
// fn -> &str { r#"Hello$0$0"#
// "##,
//             ", world",
//             10,
//         );
//         do_check(
//             r"
// #[derive($0Copy$0)]
// enum Foo {

// }
// ",
//             "Clone",
//             4,
//         );
//     }

//     #[test]
//     fn reparse_str_token_with_error_unchanged() {
//         do_check(r#""$0Unclosed$0 string literal"#, "Still unclosed", 24);
//     }

//     #[test]
//     fn reparse_str_token_with_error_fixed() {
//         do_check(r#""unterminated$0$0"#, "\"", 13);
//     }

//     #[test]
//     fn reparse_block_with_error_in_middle_unchanged() {
//         do_check(
//             r#"fn main() {
//                 if {}
//                 32 + 4$0$0
//                 return
//                 if {}
//             }"#,
//             "23",
//             105,
//         )
//     }

//     #[test]
//     fn reparse_block_with_error_in_middle_fixed() {
//         do_check(
//             r#"fn main() {
//                 if {}
//                 32 + 4$0$0
//                 return
//                 if {}
//             }"#,
//             ";",
//             105,
//         )
//     }
// }