use std::collections::HashMap;
use std::io::Write;

use lsp_types::{
    Diagnostic, DiagnosticSeverity, MessageType, NumberOrString, Range, Url 
};
use tower_lsp::Client;
use futures::future::join_all;
use tracing::{trace_span, warn};
use crate::css_language_types::LanguageSettings;
use crate::interop::typst_to_lsp::offset_to_position;
use crate::parser::css_nodes::{
    Marker, Level,
};
use crate::parser::css_parser;
use crate::workspace::FsError;


use super::CssLancerServer;

pub type DiagnosticsMap = HashMap<Url, Vec<Diagnostic>>;

impl CssLancerServer {
    #[tracing::instrument(skip(self))]
    pub async fn publish_diags(&self, url: &Url) -> Result<(), FsError> {

        let diags: Vec<Diagnostic> = {
            let src = self.source_read(url).await?;

            let l = "TREE: ".to_string() + &src.tree.fancy_string();
            self.client.log_message(MessageType::WARNING, "TREE: ".to_string() + &l).await;
            warn!(name: "TREE", tree = l);

            let mut markers = Vec::new();

            use ego_tree::NodeRef;
            use crate::parser::css_nodes::CssNode;
            fn append_issues_rec(node: &NodeRef<CssNode>, issues: &mut Vec<Marker>) {
                issues.append(&mut node.value().issues.clone());
                node.children().for_each(|ch| append_issues_rec(&ch, issues));
            }

            append_issues_rec(&src.tree.0.root(), &mut markers);

            // src.tree.0.root()
            //     .descendants()
            //     .for_each(|n| markers.append(&mut n.value().issues.clone()));

            // TODO: lints
            let m = markers.iter().map(|m| format!("{:?}", m.error)).fold("".to_string(), |acc, nex| acc + "\n" + &nex);
            warn!(name: "MARKERS", markers = m);

            let to_diagnostic = |marker: &Marker| -> Diagnostic {
                let range = Range::new(
                    offset_to_position(marker.offset, self.const_config().position_encoding.clone(), &src),
                    offset_to_position(marker.offset + marker.length, self.const_config().position_encoding.clone(), &src)
                );
                return Diagnostic::new(
                    range,
                    Some(if marker.level == Level::Warning {DiagnosticSeverity::WARNING} else {DiagnosticSeverity::ERROR}),
                    Some(NumberOrString::String(marker.error.issue().rule.id.clone())),
                    Some("csslancer".to_owned()),
                    marker.message.clone() + "|" + &marker.error.issue().rule.message,
                    None,
                    None,
                )
            };


            markers
                .into_iter()
                .filter(|marker| marker.level != Level::Ignore)
                .map(|e| to_diagnostic(&e))
                .collect()
        };
        let d = diags.iter().map(|d| d.message.clone()).fold(String::new(), |acc, nex| acc + "\n" + &nex);
        warn!(name: "MARKERS: ", diags_len = diags.len(), diags = d);
        self.client.log_message(MessageType::WARNING, "MARKERS".to_string() + &d).await;
        self.client.publish_diagnostics(url.clone(), diags, None).await;

        Ok(())
    }

}
