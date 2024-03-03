use tower_lsp::lsp_types::{Position, SemanticToken};

use crate::config::PositionEncoding;
use crate::ext::{PositionExt, StrExt};
use crate::interop::csslancer_to_lsp;
use crate::workspace::source::Source;

use super::Token;

pub(super) fn encode_tokens<'a>(
    tokens: impl Iterator<Item = Token> + 'a,
    source: &'a Source,
    encoding: PositionEncoding,
) -> impl Iterator<Item = (SemanticToken, String)> + 'a {
    tokens.scan(Position::new(0, 0), move |last_position, token| {
        let (encoded_token, source_code, position) =
            encode_token(token, last_position, source, encoding);
        *last_position = position;
        Some((encoded_token, source_code))
    })
}

fn encode_token(
    token: Token,
    last_position: &Position,
    source: &Source,
    encoding: PositionEncoding,
) -> (SemanticToken, String, Position) {
    let position = csslancer_to_lsp::offset_to_position(token.offset, encoding, source);

    let delta = last_position.delta(&position);

    let length = token.text.as_str().encoded_len(encoding);

    let lsp_token = SemanticToken {
        delta_line: delta.delta_line,
        delta_start: delta.delta_start,
        length: length as u32,
        token_type: token.token_type as u32,
        token_modifiers_bitset: 0,
    };

    (lsp_token, token.text, position)
}
