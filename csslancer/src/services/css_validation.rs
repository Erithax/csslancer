use std::collections::HashMap;

use crate::{interop::csslancer_to_client::offset_to_position, row_parser::syntax_error::SyntaxError};
use crate::workspace::FsError;
use itertools::Itertools;
use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Range, Url};
use rowan::TextSize;
use tracing::trace;

use super::CssLancerServer;

pub type DiagnosticsMap = HashMap<Url, Vec<Diagnostic>>;

impl CssLancerServer {
    #[tracing::instrument(skip(self))]
    pub async fn publish_diags(&self, url: &Url) -> Result<(), FsError> {
        let diags: Vec<Diagnostic> = {
            let src = self.source_read(url).await?;

            let errors = src.parse.errors();

            let to_diagnostic = |se: &SyntaxError| -> Diagnostic {
                let (start, end) = (se.range().start().into(), se.range().end().into());
                let range = Range::new(
                    offset_to_position(start, self.const_config().position_encoding, &src),
                    offset_to_position(end, self.const_config().position_encoding, &src),
                );
                return Diagnostic::new(
                    range,
                    Some(DiagnosticSeverity::WARNING),
                    Some(NumberOrString::String(se.to_string())),
                    Some("csslancer".to_owned()),
                    se.to_string(),
                    None,
                    None,
                );
            };

            errors
                .into_iter()
                .sorted_unstable_by_key(|se| <TextSize as Into<u32>>::into(se.range().start()))
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
