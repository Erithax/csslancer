use super::CssLancerServer;

use lsp_types::{HoverContents, MarkedString, MarkupKind, Range};
use smol_str::ToSmolStr;
use tower_lsp::lsp_types::Hover;
use tracing::trace;
use rowan::{SyntaxNode, TextSize, TokenAtOffset};

use crate::css_language_types::HoverSettings;
use crate::data::entry::{get_entry_description, IEntry2};
use crate::interop::{csslancer_to_client, client_to_csslancer, LspPosition};
use crate::row_parser::ast::AstNode;
use crate::row_parser::nodes_gen;
use crate::row_parser::{
    nodes_types::CssLanguage,
    syntax_kind_gen::SyntaxKind,
};
use crate::workspace::source::Source;

pub struct FlagOpts {
    pub text: String,
    pub is_media: bool,
}

impl CssLancerServer {
    pub fn get_hover(&self, src: &Source, position: LspPosition, settings: &Option<HoverSettings>) -> anyhow::Result<Option<Hover>> {
        let position_encoding = self.const_config().position_encoding;

        let get_lsp_range = |node: SyntaxNode<CssLanguage>| {
            let tr = node.text_range();
            Some(Range {
                start: csslancer_to_client::offset_to_position(tr.start().into(), position_encoding, src),
                end: csslancer_to_client::offset_to_position(tr.end().into(), position_encoding, src),
            })
        };

        let offset = client_to_csslancer::position_to_offset(position, position_encoding, src);
        trace!(offset = offset);
        // node path (leaf first, root last)
        fn get_node_path(sn: SyntaxNode<CssLanguage>, text_size: TextSize) -> Vec<SyntaxNode<CssLanguage>> { 
            let target = match sn.token_at_offset(text_size) {
                TokenAtOffset::Single(s) => {s},
                TokenAtOffset::Between(_a, b) => {
                    b
                }
                TokenAtOffset::None => {
                    panic!("no token?!");
                }
            };
            target.parent_ancestors().collect::<Vec<SyntaxNode<CssLanguage>>>()
        }
        let node_path = get_node_path(src.parse.tree().syntax, TextSize::new(offset.try_into().unwrap()));

        let np = node_path.clone().into_iter().fold("".to_owned(), |acc, nex| acc + &format!(">{nex:?}"));
        trace!(name: "node path", np = np);
        println!("{np}");

        let mut hover = None;
        let mut flag_opts = None;

        for syntax_node in node_path {
            if syntax_node.kind() == SyntaxKind::MEDIA {
                trace!("hovering media");
                let reggy = regex::Regex::new("@media[^{]+").unwrap(); // TODO: check regex
                let t = syntax_node.text().to_string();
                let mut matches = reggy.find_iter(&t);
                assert!(reggy.find_iter(&syntax_node.text().to_string()).count() > 0);
                flag_opts = Some(FlagOpts {
                    is_media: true,
                    text: matches.next().unwrap().as_str().to_owned()
                })
            }

            if nodes_gen::Selector::can_cast(syntax_node.kind()) {
                trace!("hovering selector");
                let selector = nodes_gen::Selector::cast(syntax_node).unwrap();
                hover = Some(Hover {
                    contents: HoverContents::Array(self.css_data_manager.selector_to_marked_string(&selector, flag_opts)),
                    range: get_lsp_range(selector.syntax),
                });
                break
            }

            if nodes_gen::SimpleSelector::can_cast(syntax_node.kind()) {
                trace!("hovering simple selector");
                let simple_selector = nodes_gen::SimpleSelector::cast(syntax_node).unwrap();
                // Some sass specific at rules such as `@at-root` are parsed as `SimpleSelector`
                if !simple_selector.syntax.text().to_string().starts_with('@') {
                    hover = Some(Hover {
                        contents: HoverContents::Array(self.css_data_manager.simple_selector_to_marked_string(&simple_selector, flag_opts)),
                        range: get_lsp_range(simple_selector.syntax),
                    });
                }
                break
            }

            if syntax_node.kind() == SyntaxKind::DECLARATION_BASIC {
                trace!("hovering declaration");
                let typed_node = nodes_gen::DeclarationBasic::cast(syntax_node).unwrap();
                let property = typed_node.property().unwrap().syntax;
                let property_name = property.text().to_string();
                if let Some(entry) = self.css_data_manager.get_property(&property_name) {
                    if let Some(contents) = get_entry_description(IEntry2::Prop(entry), self.does_support_markdown(), settings) {
                        let s = contents.value.clone();
                        trace!(message = "entry found", contents = s);
                        hover = Some(Hover {
                            contents: HoverContents::Markup(contents),
                            range: get_lsp_range(property),
                        });
                    } else {
                        hover = None;
                    }
                }
                continue;
            }

            if syntax_node.kind() == SyntaxKind::UNKNOWN_AT_RULE {
                trace!("hovering unknown at rule");
                let at_rule_name = &syntax_node.text().to_smolstr();
                if let Some(entry) = self.css_data_manager.get_at_directive(at_rule_name) {
                    if let Some(contents) = get_entry_description(IEntry2::AtDir(entry), self.does_support_markdown(), settings) {
                        hover = Some(Hover {
                            contents: HoverContents::Markup(contents),
                            range: get_lsp_range(syntax_node),
                        });
                    } else {
                        hover = None;
                    }
                }
                continue;
            }

            if syntax_node.kind() == SyntaxKind::SELECTOR_PSEUDO {
                trace!("hovering pseudoselector");
                let selector_name = syntax_node.text().to_smolstr();
                if let Some(entry) = if selector_name.starts_with("::") {
                        self.css_data_manager.get_pseudo_element(&selector_name)
                    } else {
                        self.css_data_manager.get_pseudo_class(&selector_name)
                    } 
                {
                    if let Some(contents) = get_entry_description(IEntry2::AtDir(entry), self.does_support_markdown(), settings) {
                        hover = Some(Hover {
                            contents: HoverContents::Markup(contents),
                            range: get_lsp_range(syntax_node),
                        });
                    } else {
                        hover = None;
                    }
                }
                continue;
            }
        }
        if let Some(ref mut hover) = hover {
            self.convert_contents(&mut hover.contents);
        }
        Ok(hover)
    }

    fn convert_contents(&self, hover_contents: &mut HoverContents) {
        if self.does_support_markdown() {
            return
        }
        match hover_contents {
            HoverContents::Markup(markup) => {
                // convert to plain text
                markup.kind = MarkupKind::PlainText;
            },
            HoverContents::Array(marked_string) => {
                // convert all from LanguageString to String
                for mut ms in marked_string.iter_mut() {
                    *ms = match &mut ms {
                        MarkedString::LanguageString(ref mut ls) => {
                            MarkedString::String(std::mem::take(&mut (ls).value))
                        },
                        s => {(*s).to_owned()}
                    }   
                }
            }
            HoverContents::Scalar(_) => {
                // nothing
            }
        }
    }

    fn does_support_markdown(&self) -> bool {
        return *self.client_supports_markdown.get_or_init(
            || self.client_capabilities.get()
            .map_or(false, |c| c.text_document.as_ref()
                .map_or(false, |t| t.hover.as_ref()
                    .map_or(false, |h| h.content_format.as_ref()
                        .map_or(false, |c| c.iter().any(|muk| muk == &MarkupKind::Markdown))
                    )
                )
            )
        );
	}
}




#[cfg(test)]
mod hover_test {
    
    use lsp_types::{LanguageString, Url};
    use tower_lsp::lsp_types::{Hover, HoverContents, MarkedString, MarkupContent, MarkupKind};

    use crate::{config::{ConstConfig, PositionEncoding}, css_language_types::HoverSettings, interop::csslancer_to_client, services::CssLancerServer, workspace::source::Source};

    fn assert_hover(value: &str, expected: Hover, language_id: &str, hover_settings: Option<HoverSettings>) {
        println!("assert_hover({}, {:?}, {}, {:?}", value, expected, language_id, hover_settings);
        let offset = value.find('|').expect("internal error in test: `value` must contain '|' char");
        let value = value[0..offset].to_owned() + &value[offset + 1..];
        let ls = if language_id == "css" {CssLancerServer::new_dud()}
            else if language_id == "less" {todo!()} 
            else if language_id == "scss" {todo!()} 
            else {panic!("internal error in test: `language_id` : `{language_id}` not recognized")};

        let position_encoding = PositionEncoding::Utf16;
        ls.const_config.set(ConstConfig {
            position_encoding,
            supports_semantic_tokens_dynamic_registration: false,
        }).unwrap();
        ls.client_supports_markdown.set(true).unwrap();

        let source = Source::new(Url::parse("test://foo/bar.${language_id}").unwrap(), &value, 0);

        println!("{}", source.parse.fancy_string());

        let hover_result = ls.get_hover(&source, csslancer_to_client::offset_to_position(offset, position_encoding, &source),  &hover_settings);
        assert!(hover_result.as_ref().is_ok_and(|h| h.is_some()));

        let hover_result = hover_result.unwrap().unwrap();

        if hover_result.range.is_some() && expected.range.is_some() {
            assert_eq!(hover_result.range, expected.range);
        }
        assert_eq!(hover_result.contents, expected.contents);
    }

    #[test]
    fn css_hover_basic() {
        assert_hover(
            ".test { |color: blue; }", 
            Hover {
                contents: HoverContents::Markup(MarkupContent{
                    kind: MarkupKind::Markdown,
                    value: "Sets the color of an element's text\n\n(Edge 12, Firefox 1, Safari 1, Chrome 1, IE 3, Opera 3)\n\nSyntax: &lt;color&gt;\n\n[MDN Reference](https://developer.mozilla.org/docs/Web/CSS/color)".to_owned(),
                }),
                range: None,
            },
            "css",
            Some(HoverSettings {documentation: true, references: true}),
        );
        assert_hover(
            ".test { |color: blue; }",
            Hover {
                contents: HoverContents::Markup(MarkupContent{
                    kind: MarkupKind::Markdown,
                    value: "[MDN Reference](https://developer.mozilla.org/docs/Web/CSS/color)".to_owned(),
                }),
                range: None,
            },
            "css",
            Some(HoverSettings { documentation: false, references: true}),
        );
        assert_hover(
            ".test { |color: blue; }",
            Hover {
                contents: HoverContents::Markup(MarkupContent{
                    kind: MarkupKind::Markdown,
                    value: "Sets the color of an element's text\n\n(Edge 12, Firefox 1, Safari 1, Chrome 1, IE 3, Opera 3)\n\nSyntax: &lt;color&gt;".to_owned(),
                }),
                range: None,
            },
            "css",
            Some(HoverSettings { documentation: true, references: false}),
        );

        /*
        * Reenable after converting specificity to use MarkupContent
        */

        // assert_hover(
        //     ".test:h|over { color: blue; }", 
        //     Hover {
        //         contents: HoverContents::Scalar(
        //             MarkedString::String(
        //                 "Applies while the user designates an element with a pointing device, but does not necessarily activate it. For example, a visual user agent could apply this pseudo-class when the cursor (mouse pointer) hovers over a box generated by the element.".to_owned()
        //             )
        //         ),
        //         range: None,
        //     },
        //     "css",
        //     None,
        // );

        // assert_hover(
        //     ".test::a|fter { color: blue; }", 
        //     Hover {
        // 	    contents: HoverContents::Scalar(
        //             MarkedString::String(
        //                 "Represents a styleable child pseudo-element immediately after the originating element’s actual content.".to_owned()
        //             )
        //         ),
        //         range: None,
        //     }
        //     "css",
        //     None,
        // );
    }

    #[test]
    fn css_hover_specificity() {
        assert_hover(
            ".|foo {}", 
            Hover {
                contents: HoverContents::Array(vec![
                    MarkedString::LanguageString(LanguageString{
                        language: "html".to_owned(), 
                        value: "<element class=\"foo\">".to_owned()
                    }), 
                    MarkedString::String(
                        "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (0, 1, 0)".to_owned()
                    ),
                ]),
                range: None,
            },
            "css",
            Some(HoverSettings{documentation: true, references: true}),
        );
    }

    //TODO 
    // fn scss_nested() {
    //     assert_hover(
    //         "div { d|iv {} }",
    //         Hover {
    //             contents: HoverContents::Array(vec![
    //                 MarkedString::LanguageString(LanguageString{
    //                     language: "html".to_owned(), 
    //                     value: "<div>\n  …\n    <div>".to_owned(), 
    //                 }), 
    //                 MarkedString::String(
    //                     "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (0, 0, 1)".to_owned()
    //                 )
    //             ]),
    //             range: None,
    //         },
    //         "scss",
    //         None,
    //     );
    //     assert_hover(
    //         ".foo{ .bar{ @media only screen{ .|bar{ } } } }",
    //         Hover {
    //             contents: HoverContents::Array(vec![
    //                 MarkedString::LanguageString(LanguageString{
    //                     language: "html".to_owned(),
    //                     value: "@media only screen\n … <element class=\"foo\">\n  …\n    <element class=\"bar\">\n      …\n        <element class=\"bar\">".to_owned(),
    //                 }),
    //                 MarkedString::String(
    //                     "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (0, 1, 0)".to_owned()
    //                 ),
    //             ]),
    //             range: None
    //         },
    //         "scss",
    //         None,
    //     );
    // }

    // TODO
    // fn at_root() {
    //     assert_hover(
    //         ".test { @|at-root { }",
    //         Hover {
    //             contents: HoverContents::Array(vec![]),
    //             range: None,
    //         },
    //         "scss",
    //         None,
    //     );
    // }
}