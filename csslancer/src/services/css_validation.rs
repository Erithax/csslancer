use std::collections::HashMap;

use crate::interop::csslancer_to_lsp::offset_to_position;
use crate::parser::css_nodes::{Level, Marker};
use crate::workspace::FsError;
use itertools::Itertools;
use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Range, Url};
use tracing::trace;

use super::CssLancerServer;

pub type DiagnosticsMap = HashMap<Url, Vec<Diagnostic>>;

impl CssLancerServer {
    #[tracing::instrument(skip(self))]
    pub async fn publish_diags(&self, url: &Url) -> Result<(), FsError> {
        let diags: Vec<Diagnostic> = {
            let src = self.source_read(url).await?;

            let l = "TREE: ".to_string() + &src.tree.fancy_string();
            trace!(name: "TREE", tree = l);

            let mut markers = Vec::new();

            use crate::parser::css_nodes::CssNode;
            use ego_tree::NodeRef;
            fn append_issues_rec(node: &NodeRef<CssNode>, issues: &mut Vec<Marker>) {
                issues.append(&mut node.value().issues.clone());
                node.children()
                    .for_each(|ch| append_issues_rec(&ch, issues));
            }

            append_issues_rec(&src.tree.0 .0.root(), &mut markers);

            // TODO: lints
            let m = markers
                .iter()
                .map(|m| format!("{:?}", m.error))
                .fold("".to_string(), |acc, nex| acc + "\n" + &nex);
            trace!(name: "MARKERS", markers = m);

            let to_diagnostic = |marker: &Marker| -> Diagnostic {
                let range = Range::new(
                    offset_to_position(marker.offset, self.const_config().position_encoding, &src),
                    offset_to_position(
                        marker.offset + marker.length,
                        self.const_config().position_encoding,
                        &src,
                    ),
                );
                return Diagnostic::new(
                    range,
                    Some(if marker.level == Level::Warning {
                        DiagnosticSeverity::WARNING
                    } else {
                        DiagnosticSeverity::ERROR
                    }),
                    Some(NumberOrString::String(marker.error.issue().rule.id.clone())),
                    Some("csslancer".to_owned()),
                    marker.message.clone() + "|" + &marker.error.issue().rule.message,
                    None,
                    None,
                );
            };

            markers
                .into_iter()
                .filter(|marker| marker.level != Level::Ignore)
                .sorted_unstable_by_key(|marker| marker.offset)
                .map(|e| to_diagnostic(&e))
                .collect()
        };
        let ds = format!("{diags:?}");
        trace!(name: "MARKERS: ", diags_len = diags.len(), diags = ds );
        self.client
            .publish_diagnostics(url.clone(), diags, None)
            .await;

        Ok(())
    }
}
