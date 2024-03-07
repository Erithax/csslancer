use super::CssLancerServer;

use anyhow::Context;
use lsp_types::{HoverContents, MarkedString, MarkupContent, MarkupKind, Range, TextDocumentClientCapabilities};
use tower_lsp::lsp_types::{Hover, Url};
use tower_lsp::LanguageServer;

use crate::css_language_types::HoverSettings;
use crate::data::entry::{get_entry_description, IEntry2};
use crate::parser::css_node_types::{AbstractDeclaration, AbstractDeclarationType, BodyDeclaration, BodyDeclarationType, CssNodeType, Declaration};
use crate::parser::css_nodes::{CssNode, NodeRefExt};
use crate::interop::{csslancer_to_lsp, lsp_to_csslancer, LspPosition, LspRange};

pub struct FlagOpts {
    pub text: String,
    pub is_media: bool,
}

impl CssLancerServer {
    pub async fn get_hover(&self, url: &Url, position: LspPosition, settings: &Option<HoverSettings>) -> anyhow::Result<Option<Hover>> {
        let position_encoding = self.const_config().position_encoding;

        let src = self.source_read(url).await?;

        let get_lsp_range = |node: &CssNode| {
            return Some(Range {
                start: csslancer_to_lsp::offset_to_position(node.offset, position_encoding, &src),
                end: csslancer_to_lsp::offset_to_position(node.end(), position_encoding, &src),
            })
        };

        let offset = lsp_to_csslancer::position_to_offset(position, position_encoding, &src);
        let node_path = src.tree.0.0.root().get_node_at_offset(offset);



        let mut hover = None;
        let mut flag_opts = None;

        for node in node_path {
            if node.value().node_type.same_node_type(&CssNodeType::_BodyDeclaration(BodyDeclaration {
                declarations: None,
                body_decl_type: BodyDeclarationType::Media
            })) {
                let reggy = regex::Regex::new("@media[^{]+").unwrap(); // TODO: check regex
                let mut matches = reggy.find_iter(src.tree.get_text(node.id()));
                assert!(reggy.find_iter(src.tree.get_text(node.id())).count() > 0);
                flag_opts = Some(FlagOpts {
                    is_media: true,
                    text: matches.next().unwrap().as_str().to_owned()
                })
            }

            if node.value().node_type.same_node_type(&CssNodeType::Selector) {
                hover = Some(Hover {
                    contents: HoverContents::Array(self.css_data_manager.selector_to_marked_string(&src.tree, node.id(), flag_opts)),
                    range: get_lsp_range(node.value()),
                });
                break
            }

            if node.value().node_type.same_node_type(&CssNodeType::SimpleSelector) {
                // Some sass specific at rules such as `@at-root` are parsed as `SimpleSelector`
                if !src.tree.get_text(node.id()).starts_with("@") {
                    hover = Some(Hover {
                        contents: HoverContents::Array(self.css_data_manager.selector_to_marked_string(&src.tree, node.id(), flag_opts)),
                        range: get_lsp_range(node.value()),
                    });
                }
                break
            }

            if node.value().node_type.same_node_type(&CssNodeType::_AbstractDeclaration(AbstractDeclaration {
                semicolon_position: 0,
                colon_position: 0,
                abstract_decl_type: AbstractDeclarationType::Declaration(Declaration {
                    property: node.id(),
                    expr: node.id(),
                    nested_properties: None,
                    declaration_type: crate::parser::css_node_types::DeclarationType::Declaration
                }),
            })) {
                let property_name = src.tree.get_text(node.value().node_type.unchecked_abst_decl_decl_decl_inner_ref().property);
                if let Some(entry) = self.css_data_manager.get_property(property_name) {
                    if let Some(contents) = get_entry_description(IEntry2::Prop(entry), self.does_support_markdown(), settings) {
                        hover = Some(Hover {
                            contents: HoverContents::Markup(contents),
                            range: get_lsp_range(node.value()),
                        });
                    } else {
                        hover = None;
                    }
                }
                continue;
            }

            if let CssNodeType::_BodyDeclaration(BodyDeclaration {
                declarations: None,
                body_decl_type: BodyDeclarationType::UnknownAtRule(u)
            }) = &node.value().node_type {
                let at_rule_name = src.tree.get_text(node.id());
                if let Some(entry) = self.css_data_manager.get_at_directive(at_rule_name) {
                    if let Some(contents) = get_entry_description(IEntry2::AtDir(entry), self.does_support_markdown(), settings) {
                        hover = Some(Hover {
                            contents: HoverContents::Markup(contents),
                            range: get_lsp_range(node.value()),
                        });
                    } else {
                        hover = None;
                    }
                }
                continue;
            }

            if node.value().node_type.same_node_type(&CssNodeType::PseudoSelector) {
                let selector_name = src.tree.get_text(node.id());
                if let Some(entry) = if selector_name.starts_with("::") {
                        self.css_data_manager.get_pseudo_element(selector_name)
                    } else {
                        self.css_data_manager.get_pseudo_class(selector_name)
                    } 
                {
                    if let Some(contents) = get_entry_description(IEntry2::AtDir(entry), self.does_support_markdown(), settings) {
                        hover = Some(Hover {
                            contents: HoverContents::Markup(contents),
                            range: get_lsp_range(node.value()),
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
        return Ok(hover)
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
                for ms in marked_string.iter_mut() {
                    *ms = match ms {
                        MarkedString::LanguageString(ls) => {
                            MarkedString::String(ls.value)
                        },
                        s => {*s}
                    }   
                }
            }
            HoverContents::Scalar(scalar) => {
                
            }
        }
    }

    fn does_support_markdown(&self) -> bool {
        return *self.client_supports_markdown.get_or_init(
            || self.client_capabilities.get()
            .map_or(false, |c| c.text_document
                .map_or(false, |t| t.hover
                    .map_or(false, |h| h.content_format
                        .map_or(false, |c| c.iter().any(|muk| muk == &MarkupKind::Markdown))
                    )
                )
            )
        );
	}
}