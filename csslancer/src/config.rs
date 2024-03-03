use anyhow::bail;
use futures::future::BoxFuture;
use itertools::Itertools;
use serde::Deserialize;
use serde_json::{Map, Value};
use tower_lsp::lsp_types::{self, ConfigurationItem, InitializeParams, PositionEncodingKind};

use crate::ext::InitializeParamsExt;

#[derive(Debug, Clone, Copy)]
pub enum PositionEncoding {
    Utf16,
    Utf8,
}

impl From<PositionEncoding> for lsp_types::PositionEncodingKind {
    fn from(position_encoding: PositionEncoding) -> Self {
        match position_encoding {
            PositionEncoding::Utf16 => Self::UTF16,
            PositionEncoding::Utf8 => Self::UTF8,
        }
    }
}

/// Configuration set at initialization that won't change within a single session
#[derive(Debug)]
pub struct ConstConfig {
    pub position_encoding: PositionEncoding,
    pub supports_semantic_tokens_dynamic_registration: bool,
    // pub supports_document_formatting_dynamic_registration: bool,
    // pub supports_config_change_registration: bool,
}

impl ConstConfig {
    fn choose_encoding(params: &InitializeParams) -> PositionEncoding {
        let encodings = params.position_encodings();
        if encodings.contains(&PositionEncodingKind::UTF8) {
            PositionEncoding::Utf8
        } else {
            PositionEncoding::Utf16
        }
    }
}

impl From<&InitializeParams> for ConstConfig {
    fn from(params: &InitializeParams) -> Self {
        Self {
            position_encoding: Self::choose_encoding(params),
            supports_semantic_tokens_dynamic_registration: params
                .supports_semantic_tokens_dynamic_registration(),
            // supports_document_formatting_dynamic_registration: params
            //     .supports_document_formatting_dynamic_registration(),
            // supports_config_change_registration: params.supports_config_change_registration(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SemanticTokensMode {
    Disable,
    #[default]
    Enable,
}

pub type Listener<T> = Box<dyn FnMut(&T) -> BoxFuture<anyhow::Result<()>> + Send + Sync>;

const CONFIG_ITEMS: &[&str] = &["semanticTokens"];

#[derive(Default)]
pub struct Config {
    // pub main_file: Option<Url>,
    // pub export_pdf: ExportPdfMode,
    // pub root_path: Option<PathBuf>,
    pub semantic_tokens: SemanticTokensMode,
    // pub formatter: ExperimentalFormatterMode,
    semantic_tokens_listeners: Vec<Listener<SemanticTokensMode>>,
    // formatter_listeners: Vec<Listener<ExperimentalFormatterMode>>,
}

impl Config {
    pub fn get_items() -> Vec<ConfigurationItem> {
        let sections = CONFIG_ITEMS
            .iter()
            .flat_map(|item| [format!("csslancer.{item}"), item.to_string()]);

        sections
            .map(|section| ConfigurationItem {
                section: Some(section),
                ..Default::default()
            })
            .collect()
    }

    pub fn values_to_map(values: Vec<Value>) -> Map<String, Value> {
        let unpaired_values = values
            .into_iter()
            .tuples()
            .map(|(a, b)| if !a.is_null() { a } else { b });

        CONFIG_ITEMS
            .iter()
            .map(|item| item.to_string())
            .zip(unpaired_values)
            .collect()
    }

    pub fn listen_semantic_tokens(&mut self, listener: Listener<SemanticTokensMode>) {
        self.semantic_tokens_listeners.push(listener);
    }

    pub async fn update(&mut self, update: &Value) -> anyhow::Result<()> {
        if let Value::Object(update) = update {
            self.update_by_map(update).await
        } else {
            bail!("got invalid configuration object {update}")
        }
    }

    pub async fn update_by_map(&mut self, update: &Map<String, Value>) -> anyhow::Result<()> {
        let semantic_tokens = update
            .get("semanticTokens")
            .map(SemanticTokensMode::deserialize)
            .and_then(Result::ok);
        if let Some(semantic_tokens) = semantic_tokens {
            for listener in &mut self.semantic_tokens_listeners {
                listener(&semantic_tokens).await?;
            }
            self.semantic_tokens = semantic_tokens;
        }

        Ok(())
    }
}
