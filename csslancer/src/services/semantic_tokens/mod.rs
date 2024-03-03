use ego_tree::NodeId;
use itertools::Itertools;
use strum::IntoEnumIterator;
use tower_lsp::lsp_types::{
    Registration, SemanticToken, SemanticTokensEdit, SemanticTokensFullOptions,
    SemanticTokensLegend, SemanticTokensOptions, Unregistration,
};

use crate::parser::css_nodes::{CssNode, CssNodeTree};
use crate::workspace::source::Source;

use self::csslancer_tokens::TokenType;
use self::delta::token_delta;
use self::token_encode::encode_tokens;

use super::CssLancerServer;

pub use self::delta::Cache as SemanticTokenCache;

mod csslancer_tokens;
mod delta;
mod token_encode;

pub fn get_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: TokenType::iter().map(Into::into).collect(),
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

        let tokens = tokenize_tree(&source.tree);

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

fn tokenize_node_rec(tree: &CssNodeTree, node_id: NodeId) -> Box<dyn Iterator<Item = Token> + '_> {
    let node = tree.0.get(node_id).unwrap();
    let is_leaf = node.has_children();

    let node_token = token_type_from_node(node.value())
        .or_else(|| is_leaf.then_some(TokenType::Text))
        .map(|token_type| {
            Token::new(
                token_type,
                node.value().offset,
                tree.get_text(node_id).to_owned(),
            )
        })
        .into_iter();

    let children = node
        .children()
        .flat_map(|ch| tokenize_node_rec(tree, ch.id()));

    Box::new(node_token.chain(children))
}

/// Tokenize a node and its children
fn tokenize_tree(tree: &CssNodeTree) -> Box<dyn Iterator<Item = Token> + '_> {
    return tokenize_node_rec(tree, tree.0 .0.root().id());
}

/// `offset` in csslancer (Utf-8) space
#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub offset: usize,
    pub text: String,
}

impl Token {
    pub fn new(token_type: TokenType, offset: usize, text: String) -> Self {
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
fn token_type_from_node(node: &CssNode) -> Option<TokenType> {
    use crate::parser::css_node_types::CssNodeType::*;

    match node.node_type {
        Undefined
        | ROOT
        | Nodelist
        | _BodyDeclaration(..)
        | _AbstractDeclaration(..)
        | _Invocation(..)
        | Stylesheet
        | Selector
        | SimpleSelector
        | SelectorInterpolation
        | Declarations => None,
        SelectorCombinator
        | SelectorCombinatorParent
        | SelectorCombinatorSibling
        | SelectorCombinatorAllSiblings
        | SelectorCombinatorShadowPiercingDescendant => Some(TokenType::Operator),
        Identifier(..) => Some(TokenType::Ref),
        ClassSelector | IdentifierSelector | PseudoSelector | AttributeSelector(..) => {
            Some(TokenType::Punctuation)
        }
        ElementNameSelector => Some(TokenType::ElementName),
        Property(..) => Some(TokenType::Property),
        Expression => None,
        Operator => Some(TokenType::Operator),
        StringLiteral | URILiteral => Some(TokenType::String),
        EscapedValue => Some(TokenType::Escape),
        NumericValue | HexColorValue | RatioValue => Some(TokenType::Number),
        Prio => Some(TokenType::Important),
        Interpolation => Some(TokenType::Interpolated),

        ExtendsReference
        | SelectorPlaceholder
        | Debug
        | MixinContentReference
        | Import
        | Namespace
        | ReturnStatement
        | MediaQuery
        | MediaCondition
        | MediaFeature
        | FunctionParameter
        | FunctionArgument(..)
        | AtApplyRule
        | ListEntry
        | SupportsCondition(..)
        | NamespacePrefix
        | GridLine
        | Plugin
        | Use
        | ModuleConfiguration
        | Forward
        | ForwardVisibility
        | Module
        | UnicodeRange(..)
        | LayerNameList
        | LayerName
        | VariableDeclaration(..)
        | VariableName
        | MixinReference(..)
        | BinaryExpression(..)
        | Term(..)
        | Value
        | LessGuard
        | Variable
        | CustomPropertyValue
        | Medialist => None,
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
