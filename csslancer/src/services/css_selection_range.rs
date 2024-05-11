use std::ops::Range;

use lsp_types::SelectionRange;
use rowan::{SyntaxElement, TextSize};

use crate::{
    interop::{csslancer_to_client, client_to_csslancer, LspPosition, LspPositionEncoding},
    row_parser::{
        self,
        syntax_kind_gen::SyntaxKind,
    },
    workspace::source::Source,
};

use super::CssLancerServer;

impl CssLancerServer {
    pub fn get_selection_ranges(
        &self,
        source: &Source,
        positions: &[LspPosition],
    ) -> Vec<SelectionRange> {
        let lsp_pos_enc = self.const_config().position_encoding;
        return positions
            .iter()
            .map(|pos| Self::get_selection_range(source, *pos, lsp_pos_enc))
            .collect();
    }

    #[cfg(test)]
    pub fn get_selection_ranges_with_enc(
        &self,
        source: &Source,
        positions: &[LspPosition],
        position_encoding: crate::config::PositionEncoding,
    ) -> Vec<SelectionRange> {
        return positions
            .iter()
            .map(|pos| Self::get_selection_range(source, *pos, position_encoding))
            .collect();
    }

    pub fn get_selection_range(
        source: &Source,
        position: LspPosition,
        lsp_pos_enc: LspPositionEncoding,
    ) -> SelectionRange {
        let offset = client_to_csslancer::position_to_offset(position, lsp_pos_enc, source);

        let target = match source.parse.syntax_node().token_at_offset(TextSize::new(offset.try_into().unwrap())) {
            rowan::TokenAtOffset::Single(s) => s,
            rowan::TokenAtOffset::Between(a, b) => 
                if a.kind().is_trivia() || (a.kind().is_punct() && !b.kind().is_punct()) {
                    b
                } else if b.kind().is_trivia() || (b.kind().is_punct() && !a.kind().is_punct()) {
                    a
                } else {
                    b
                }
            rowan::TokenAtOffset::None => panic!("offset in document with no token!? {}", offset),
        };

        let applicable_ranges =
            {
                let mut res = Vec::new();
                let mut curr_node_opt: Option<rowan::NodeOrToken<rowan::SyntaxNode<row_parser::nodes_types::CssLanguage>, rowan::SyntaxToken<row_parser::nodes_types::CssLanguage>>> = Some(SyntaxElement::Token(target));
                while let Some(ref curr_node) = curr_node_opt {
                    if let Some(par) = curr_node.parent() {
                        if <TextSize as Into<u32>>::into(curr_node.text_range().start()) == <TextSize as Into<u32>>::into(par.text_range().start())
                            && curr_node.text_range().len() == par.text_range().len()
                        {
                            curr_node_opt = Some(SyntaxElement::Node(par));
                            continue;
                        }
                    }

                    if matches!(curr_node.kind(), SyntaxKind::L_CURLY | SyntaxKind::R_CURLY | SyntaxKind::WHITESPACE) {
                        curr_node_opt = curr_node.parent().map(SyntaxElement::Node);
                        continue
                    }

                    // The `{ }` part of `.a { }`
                    if curr_node.kind() == SyntaxKind::DECLARATIONS
                        && offset > curr_node.text_range().start().into()
                        && offset < curr_node.text_range().end().into()
                    {
                        // Return `{ }` and the range inside `{` and `}`
                        res.push((<TextSize as Into<u32>>::into(curr_node.text_range().start()) + 1, <TextSize as Into<u32>>::into(curr_node.text_range().end()) - 1));
                    }

                    res.push((curr_node.text_range().start().into(), curr_node.text_range().end().into()));

                    curr_node_opt = curr_node.parent().map(SyntaxElement::Node);
                }
                res
            };

        let mut current = None;
        for app_range in applicable_ranges.iter().rev() {
            current = Some(SelectionRange {
                range: csslancer_to_client::range(
                    Range {
                        start: app_range.0 as usize,
                        end: app_range.1 as usize,
                    },
                    source,
                    lsp_pos_enc,
                )
                .raw_client_range,
                parent: current.map(Box::new),
            })
        }

        if let Some(current) = current {
            return current;
        }
        SelectionRange {
            range: lsp_types::Range {
                start: position,
                end: position,
            },
            parent: None,
        }
    }
}





/**
* We don"t do much testing since as long as the parser generates a valid AST,
* correct selection ranges will be generated.
*/
#[cfg(test)]
mod css_selection_range_test {

    use lsp_types::Url;

    use crate::config::PositionEncoding;
    use crate::interop::ClientRange;
    use crate::services::CssLancerServer;
    use crate::workspace::source::Source;
    use crate::services::selection_range::SelectionRange;
    use crate::interop::{csslancer_to_client::offset_to_position, client_to_csslancer::position_to_offset, client_to_csslancer};


    fn assert_ranges(content: &str, expected: &[(usize, &str)]) {
        let mut message = format!("{content} gives selection range:\n");
    
        let offset = content.find("|").unwrap();
        let content = content[0..offset].to_owned() + &content[offset+'|'.len_utf8()..];
    
        let ls = CssLancerServer::new_dud();
        
        let position_encoding = PositionEncoding::Utf16;

        let source = Source::new(Url::parse("test://foo/bar.css").unwrap(), &content, 0);
        let actual_ranges = ls.get_selection_ranges_with_enc(
            &source, 
            &[offset_to_position(offset, position_encoding, &source)],
            position_encoding,
        );
        assert_eq!(actual_ranges.len(), 1);
        let mut offset_pairs = Vec::new();
        let mut curr_opt: Option<Box<SelectionRange>> = Some(Box::new(actual_ranges.into_iter().next().unwrap()));
        while let Some(curr) = curr_opt {
            let client_range = ClientRange::new(curr.range, position_encoding);
            offset_pairs.push((
                position_to_offset(curr.range.start, position_encoding, &source), 
                source.text_at(client_to_csslancer::range(&client_range, &source)).to_string()
            ));
            curr_opt = curr.parent;
        }
    
        message += &format!("{offset_pairs:?}\n but should give:\n{expected:?}\n");
        assert_eq!(offset_pairs.iter().map(|op| (op.0, op.1.as_str())).collect::<Vec<(usize, &str)>>(), expected, "{message}");
    }

    #[test]
    fn basic() {
        assert_ranges(".foo { |color: blue; }", &[
            (7, "color"),
            (7, "color: blue"),
            (6, " color: blue; "),
            (5, "{ color: blue; }"),
            (0, ".foo { color: blue; }"),
        ]);
        assert_ranges(".foo { c|olor: blue; }", &[
            (7, "color"),
            (7, "color: blue"),
            (6, " color: blue; "),
            (5, "{ color: blue; }"),
            (0, ".foo { color: blue; }"),
        ]);
        assert_ranges(".foo { color|: blue; }", &[
            (7, "color"),
            (7, "color: blue"),
            (6, " color: blue; "),
            (5, "{ color: blue; }"),
            (0, ".foo { color: blue; }"),
        ]);

        assert_ranges(".foo { color: |blue; }", &[
            (14, "blue"),
            (7, "color: blue"),
            (6, " color: blue; "),
            (5, "{ color: blue; }"),
            (0, ".foo { color: blue; }"),
        ]);
        assert_ranges(".foo { color: b|lue; }", &[
            (14, "blue"),
            (7, "color: blue"),
            (6, " color: blue; "),
            (5, "{ color: blue; }"),
            (0, ".foo { color: blue; }"),
        ]);
        assert_ranges(".foo { color: blue|; }", &[
            (14, "blue"),
            (7, "color: blue"),
            (6, " color: blue; "),
            (5, "{ color: blue; }"),
            (0, ".foo { color: blue; }"),
        ]);

        assert_ranges(".|foo { color: blue; }", &[
            (1, "foo"),
            (0, ".foo"),
            (0, ".foo { color: blue; }"),
        ]);
        assert_ranges(".fo|o { color: blue; }", &[
            (1, "foo"),
            (0, ".foo"),
            (0, ".foo { color: blue; }"),
        ]);
        assert_ranges(".foo| { color: blue; }", &[
            (1, "foo"),
            (0, ".foo"),
            (0, ".foo { color: blue; }"),
        ]);
    }

    #[test]
    fn multiple_value() {
        assert_ranges(".foo { font-family: \"|Courier New\", Courier, monospace; }", &[
            (20, "\"Courier New\""),
            (20, "\"Courier New\", Courier, monospace"),
            (7, "font-family: \"Courier New\", Courier, monospace"),
            (6, " font-family: \"Courier New\", Courier, monospace; "),
            (5, "{ font-family: \"Courier New\", Courier, monospace; }"),
            (0, ".foo { font-family: \"Courier New\", Courier, monospace; }"),
        ]);
    }

    // https://github.com/microsoft/vscode/issues/83570
    #[test]
    fn edge_behavious_for_declaration() {
        assert_ranges(".foo |{ }", &[
            (5, "{ }"),
            (0, ".foo { }"),
        ]);
        assert_ranges(".foo { }|", &[
            (5, "{ }"),
            (0, ".foo { }"),
        ]);
        assert_ranges(".foo {| }", &[
            (6, " "),
            (5, "{ }"),
            (0, ".foo { }"),
        ]);
        assert_ranges(".foo { | }", &[
            (6, "  "),
            (5, "{  }"),
            (0, ".foo {  }"),
        ]);
        assert_ranges(".foo { |}", &[
            (6, " "),
            (5, "{ }"),
            (0, ".foo { }"),
        ]);
    }
}