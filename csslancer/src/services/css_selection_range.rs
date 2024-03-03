use std::ops::Range;

use lsp_types::SelectionRange;

use crate::{
    interop::{csslancer_to_lsp, lsp_to_csslancer, LspPosition, LspPositionEncoding},
    parser::{css_node_types::CssNodeType, css_nodes::ChildByOffsetFinder},
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

    pub fn get_selection_range(
        source: &Source,
        position: LspPosition,
        lsp_pos_enc: LspPositionEncoding,
    ) -> SelectionRange {
        let offset = lsp_to_csslancer::position_to_offset(position, lsp_pos_enc, source);

        let applicable_ranges =
            if let Some(curr_node) = source.tree.0 .0.root().find_child_at_offset(offset, true) {
                let mut res = Vec::new();
                let mut curr_node_opt = Some(curr_node);
                while let Some(curr_node) = curr_node_opt {
                    if let Some(par) = curr_node.parent() {
                        if curr_node.value().offset == par.value().offset
                            && curr_node.value().length == par.value().length
                        {
                            curr_node_opt = Some(par);
                            continue;
                        }
                    }

                    // The `{ }` part of `.a { }`
                    if curr_node
                        .value()
                        .node_type
                        .same_node_type(&CssNodeType::Declarations)
                        && offset > curr_node.value().offset
                        && offset < curr_node.value().end()
                    {
                        // Return `{ }` and the range inside `{` and `}`
                        res.push((curr_node.value().offset + 1, curr_node.value().end() - 1));
                    }

                    res.push((curr_node.value().offset, curr_node.value().end()));

                    curr_node_opt = curr_node.parent();
                }
                res
            } else {
                Vec::new()
            };

        let mut current = None;
        for app_range in applicable_ranges.iter().rev() {
            current = Some(SelectionRange {
                range: csslancer_to_lsp::range(
                    Range {
                        start: app_range.0,
                        end: app_range.1,
                    },
                    source,
                    lsp_pos_enc,
                )
                .raw_range,
                parent: current.map(Box::new),
            })
        }

        if let Some(current) = current {
            return current;
        }
        return SelectionRange {
            range: lsp_types::Range {
                start: position,
                end: position,
            },
            parent: None,
        };
    }
}
