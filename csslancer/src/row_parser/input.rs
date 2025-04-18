//! See [`Input`].

use super::syntax_kind_gen::SyntaxKind;

#[allow(non_camel_case_types)]
type bits = u64;

/// Input for the parser -- a sequence of tokens.
///
/// As of now, parser doesn't have access to the *text* of the tokens, and makes
/// decisions based solely on their classification. Unlike `LexerToken`, the
/// `Tokens` doesn't include whitespace and comments. Main input to the parser.
///
/// Struct of arrays internally, but this shouldn't really matter.
#[derive(Default)]
pub struct Input {
    kind: Vec<SyntaxKind>,
    joint: Vec<bits>,
    contextual_kind: Vec<SyntaxKind>,
}

/// `pub` impl used by callers to create `Tokens`.
impl Input {
    #[inline]
    pub fn push(&mut self, kind: SyntaxKind) {
        self.push_impl(kind, SyntaxKind::EOF)
    }
    #[inline]
    pub fn push_ident(&mut self, contextual_kind: SyntaxKind) {
        self.push_impl(SyntaxKind::IDENTIFIER, contextual_kind)
    }
    #[inline]
    pub fn push_func(&mut self, contextual_kind: SyntaxKind) {
        // to handle contextual style()
        self.push_impl(SyntaxKind::FUNCTION, contextual_kind)
    }
    #[inline]
    pub fn push_id_hash(&mut self, contextual_kind: SyntaxKind) {
        // to handle contextual hex
        self.push_impl(SyntaxKind::ID_HASH, contextual_kind)
    }
    #[inline]
    pub fn push_unrestriced_hash(&mut self, contextual_kind: SyntaxKind) {
        // to handle contextual hex
        self.push_impl(SyntaxKind::UNRESTRICTED_HASH, contextual_kind)
    }
    #[inline]
    pub fn push_unknown_dimension(&mut self, contextual_kind: SyntaxKind) {
        // to handle contextual hex
        self.push_impl(SyntaxKind::DIM_UNKNOWN, contextual_kind)
    }
    /// Sets had_whitespace for the last token we've pushed. Denotes there was whitespace after the token.
    ///
    /// Jointness is stored as a `Vec<u64>`, i.e. the jointness of the first
    /// 64 `SyntaxKind`s is stored in the first `u64`.
    /// 
    /// This is a separate API rather than an argument to the `push` to make it
    /// convenient both for textual and mbe tokens. With text, you know whether
    /// the *previous* token was joint, with mbe, you know whether the *current*
    /// one is joint. This API allows for styles of usage:
    ///
    /// ```ignore
    /// // In text:
    /// tokens.was_joint(prev_joint);
    /// tokens.push(curr);
    ///
    /// // In MBE:
    /// token.push(curr);
    /// tokens.push(curr_joint)
    /// ```
    #[inline]
    pub fn had_whitespace(&mut self) {
        debug_assert!(self.len() > 0);
        let n = self.len() - 1;
        let (idx, b_idx) = self.bit_index(n);
        self.joint[idx] |= 1 << b_idx;
    }
    #[inline]
    fn push_impl(&mut self, kind: SyntaxKind, contextual_kind: SyntaxKind) {
        let idx = self.len();
        if idx % (bits::BITS as usize) == 0 {
            self.joint.push(0);
        }
        self.kind.push(kind);
        self.contextual_kind.push(contextual_kind);
    }
}

/// pub(crate) impl used by the parser to consume `Tokens`.
impl Input {
    pub(crate) fn last_kind(&self) -> SyntaxKind {
        *self.kind.last().unwrap_or(&SyntaxKind::EOF)
    }

    pub(crate) fn kind(&self, idx: usize) -> SyntaxKind {
        self.kind.get(idx).copied().unwrap_or(SyntaxKind::EOF)
    }
    pub(crate) fn contextual_kind(&self, idx: usize) -> SyntaxKind {
        self.contextual_kind.get(idx).copied().unwrap_or(SyntaxKind::EOF)
    }
    pub(crate) fn has_whitespace_after(&self, n: usize) -> bool {
        let (idx, b_idx) = self.bit_index(n);
        (self.joint[idx] & (1 << b_idx)) != 0
    }
}

impl Input {
    /// Returns (idx = index into self.joint, b_idx = index into u64 at self.joint[idx])
    fn bit_index(&self, n: usize) -> (usize, usize) {
        let idx = n / (bits::BITS as usize);
        let b_idx = n % (bits::BITS as usize);
        (idx, b_idx)
    }
    pub(crate)fn len(&self) -> usize {
        self.kind.len()
    }
}
