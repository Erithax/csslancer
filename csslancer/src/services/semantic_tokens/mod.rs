use itertools::Itertools;
use strum::IntoEnumIterator;
use tower_lsp::lsp_types::{
    Registration, SemanticToken, SemanticTokensEdit, SemanticTokensFullOptions,
    SemanticTokensLegend, SemanticTokensOptions, Unregistration,
};
use rowan::SyntaxNode;

use crate::{row_parser::nodes_types::CssLanguage, workspace::source::Source};

use self::csslancer_tokens::SemTokenKind;
use self::delta::token_delta;
use self::token_encode::encode_tokens;
use crate::row_parser::{
    syntax_kind_gen::SyntaxKind,
};

use super::CssLancerServer;

pub use self::delta::Cache as SemanticTokenCache;

mod csslancer_tokens;
mod delta;
mod token_encode;

pub fn get_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: SemTokenKind::iter().map(Into::into).collect(),
        token_modifiers: Vec::new(),
    }
}

const SEMANTIC_TOKENS_REGISTRATION_ID: &str = "semantic_tokens";
const SEMANTIC_TOKENS_METHOD_ID: &str = "textDocument/semanticTokens";

pub fn get_semantic_tokens_registration(options: SemanticTokensOptions) -> Registration {
    Registration {
        id: SEMANTIC_TOKENS_REGISTRATION_ID.to_owned(),
        method: SEMANTIC_TOKENS_METHOD_ID.to_owned(),
        register_options: Some(
            serde_json::to_value(options)
                .expect("semantic tokens options should be representable as JSON value"),
        ),
    }
}

pub fn get_semantic_tokens_unregistration() -> Unregistration {
    Unregistration {
        id: SEMANTIC_TOKENS_REGISTRATION_ID.to_owned(),
        method: SEMANTIC_TOKENS_METHOD_ID.to_owned(),
    }
}

pub fn get_semantic_tokens_options() -> SemanticTokensOptions {
    SemanticTokensOptions {
        legend: get_legend(),
        full: Some(SemanticTokensFullOptions::Delta { delta: Some(true) }),
        ..Default::default()
    }
}

impl CssLancerServer {
    #[tracing::instrument(skip(self))]
    pub fn get_semantic_tokens_full(&self, source: &Source) -> (Vec<SemanticToken>, String) {
        let encoding = self.const_config().position_encoding;

        let tokens = tokenize_tree(source.parse.syntax_node());

        let encoded_tokens = encode_tokens(tokens, source, encoding);
        let output_tokens = encoded_tokens.map(|(token, _)| token).collect_vec();

        let result_id: String = self
            .semantic_tokens_delta_cache
            .write()
            .unwrap()
            .cache_result(output_tokens.clone());

        (output_tokens, result_id)
    }

    pub fn try_semantic_tokens_delta_from_result_id(
        &self,
        source: &Source,
        result_id: &str,
    ) -> (Result<Vec<SemanticTokensEdit>, Vec<SemanticToken>>, String) {
        let cached = self
            .semantic_tokens_delta_cache
            .write()
            .unwrap()
            .try_take_result(result_id);

        // this call will overwrite the cache, so need to read from cache first
        let (tokens, result_id) = self.get_semantic_tokens_full(source);

        match cached {
            Some(cached) => (Ok(token_delta(&cached, &tokens)), result_id),
            None => (Err(tokens), result_id),
        }
    }
}

fn tokenize_node_rec(syntax_node: SyntaxNode<CssLanguage>) -> Box<dyn Iterator<Item = Token>> {
    let is_leaf = syntax_node.children().count() == 0;

    let node_token: std::option::IntoIter<Token> = sem_token_kind_from_syntax_node(&syntax_node)
        .or_else(|| is_leaf.then_some(SemTokenKind::Text))
        .map(|token_type| {
            Token::new(
                token_type,
                syntax_node.text_range().start().into(),
                syntax_node.text().to_string(),
            )
        })
        .into_iter();

    let children = syntax_node
        .children()
        .flat_map(tokenize_node_rec);

    Box::new(node_token.chain(children))
}

/// Tokenize a node and its children
fn tokenize_tree(syntax_node: SyntaxNode<CssLanguage>) -> Box<dyn Iterator<Item = Token>> {
    tokenize_node_rec(syntax_node)
}

/// `offset` in csslancer (Utf-8) space
#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: SemTokenKind,
    pub offset: usize,
    pub text: String,
}

impl Token {
    pub fn new(token_type: SemTokenKind, offset: usize, text: String) -> Self {
        Self {
            token_type,
            offset,
            text,
        }
    }
}

/// Determines the best [`TokenType`] for an entire node and its children, if any. If there is no
/// single `TokenType`, or none better than `Text`, returns `None`.
///
/// In tokenization, returning `Some` stops recursion, while returning `None` continues and attempts
/// to tokenize each of `node`'s children. If there are no children, `Text` is taken as the default.
fn sem_token_kind_from_syntax_node(syntax_node: &SyntaxNode<CssLanguage>) -> Option<SemTokenKind> {
    use SyntaxKind::*;

    match syntax_node.kind() {
        SELECTOR_COMBINATOR
        | SELECTOR_COMBINATOR_ALL_SIBLINGS
        | SELECTOR_COMBINATOR_PARENT
        | SELECTOR_COMBINATOR_SHADOW_PIERCING_DESCENDANT
        | SELECTOR_COMBINATOR_SIBLING
        | OPERATOR_DASHMATCH
        | OPERATOR_INCLUDES
        | OPERATOR_PREFIX
        | OPERATOR_SUFFIX
        | OPERATOR_SUBSTRING
        | STAR
        | PLUS
        | SLASH
        | MINUS
        | EQ => Some(SemTokenKind::Operator),
        IDENTIFIER => Some(SemTokenKind::Identifier),
        R_CURLY |
        L_CURLY |
        R_PAREN | 
        L_PAREN |
        R_BRACK |
        L_BRACK |
        DOT |
        COMMA |
        SEMICOLON |
        COLON => Some(SemTokenKind::Punctuation),
        PROPERTY => Some(SemTokenKind::Property),
        STRING | URL | BAD_STRING | BAD_URL => Some(SemTokenKind::String),
        NUMERIC_VALUE | HEX_COLOR_VALUE | RATIO_VALUE  => Some(SemTokenKind::Number),
        k if k.is_dimension() => Some(SemTokenKind::Number),
        PRIO => Some(SemTokenKind::Important),
        _ => {None}
    }
}

// TODO: differentiate also using tokens in scope, not just context
// fn is_function_ident(ident: &LinkedNode) -> bool {
//     let Some(next) = ident.next_leaf() else {
//         return false;
//     };
//     let function_call = matches!(next.kind(), SyntaxKind::LeftParen)
//         && matches!(
//             next.parent_kind(),
//             Some(SyntaxKind::Args | SyntaxKind::Params)
//         );
//     let function_content = matches!(next.kind(), SyntaxKind::LeftBracket)
//         && matches!(next.parent_kind(), Some(SyntaxKind::ContentBlock));
//     function_call || function_content
// }

// fn token_from_ident(ident: &CssNode) -> TokenType {
//     if is_function_ident(ident) {
//         TokenType::Function
//     } else {
//         TokenType::Interpolated
//     }
// }

// fn get_expr_following_hashtag<'a>(hashtag: &CssNode<'a>) -> Option<CssNode<'a>> {
//     hashtag
//         .next_sibling()
//         .filter(|next| next.cast::<ast::Expr>().map_or(false, |expr| expr.hash()))
//         .and_then(|node| node.leftmost_leaf())
// }

// fn token_from_hashtag(hashtag: &LinkedNode) -> Option<TokenType> {
//     get_expr_following_hashtag(hashtag)
//         .as_ref()
//         .and_then(token_from_node)
// }
