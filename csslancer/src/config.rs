

use lsp_types::PositionEncodingKind;
use tower_lsp::lsp_types::InitializeParams;

use crate::ext::InitializeParamsExt;





#[derive(Debug, Clone, Copy)]
pub enum PositionEncoding {
    Utf16,
    Utf8
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
    // pub supports_semantic_tokens_dynamic_registration: bool,
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
            // supports_semantic_tokens_dynamic_registration: params
            //     .supports_semantic_tokens_dynamic_registration(),
            // supports_document_formatting_dynamic_registration: params
            //     .supports_document_formatting_dynamic_registration(),
            // supports_config_change_registration: params.supports_config_change_registration(),
        }
    }
}
