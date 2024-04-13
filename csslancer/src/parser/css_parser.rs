use ego_tree::{NodeId, NodeMut, NodeRef};
use regex::Regex;
use regex::RegexBuilder;

use crate::data::facts;
use crate::parser::css_error::*;
use crate::parser::css_node_types::*;
use crate::parser::css_nodes::*;
use crate::parser::css_scanner::*;
use crate::workspace::source::Source;
use csslancer_macro::addchild;
use csslancer_macro::addchildbool;

#[derive(Debug, Clone)]
pub struct Mark {
    prev: Option<Token>,
    curr: Token,
    pos: usize,
    last_err: Option<Token>,
}

// impl ITextProvider for TextDocumentItem {
//     fn get_text(&self, offset: usize, length: usize) -> &str {
//         if self.version != self.version {
//             panic!("Underlying model has changed, AST is no longert valid");
//         }
//         return &self.text[offset..length];
//     }
// }

impl ITextProvider for &Source {
    fn get_text(&self, offset: usize, length: usize) -> &str {
        // TODO? version check
        return &self.text()[offset..length];
    }
}

#[derive(Debug)]
pub struct TextFromStringProvider<'a> {
    inner: &'a str,
}

impl ITextProvider for TextFromStringProvider<'_> {
    fn get_text(&self, offset: usize, length: usize) -> &str {
        return &self.inner[offset..length];
    }
}

/// <summary>
/// A parser for the css core specification. See for reference:
/// https://www.w3.org/TR/CSS21/grammar.html
/// http://www.w3.org/TR/CSS21/syndata.html#tokenization
/// </summary>
pub struct Parser {
    pub scanner: Scanner,
    pub token: Token,
    pub prev_token: Option<Token>,
    last_error_token: Option<Token>,
    pub tree: SourceLessCssNodeTree,
    root: NodeId,
}

impl Parser {
    pub fn new(mut scanner: Scanner) -> Self {
        let token = scanner.scan();
        let tree =
            SourceLessCssNodeTree::new(CssNode::new(0, scanner.stream.length, CssNodeType::ROOT));
        return Self {
            scanner,
            token,
            prev_token: None,
            last_error_token: None,
            root: tree.0.root().id(),
            tree,
        };
    }

    pub fn new_with_text(text: String) -> Self {
        let mut scanner = Scanner::default();
        let tree = SourceLessCssNodeTree::new(CssNode::new(0, text.len(), CssNodeType::ROOT));
        scanner.set_source(text);
        let token = scanner.scan();
        return Self {
            scanner,
            token,
            prev_token: None,
            last_error_token: None,
            root: tree.0.root().id(),
            tree,
        };
    }

    pub fn take_source(&mut self) -> String {
        return self.scanner.stream.take_source();
    }

    pub fn take_tree(&mut self) -> SourceLessCssNodeTree {
        return std::mem::take(&mut self.tree);
    }

    // =======================
    // MARK (ability to save state of self and restore it)
    // =======================

    pub fn mark(&self) -> Mark {
        return Mark {
            prev: self.prev_token.clone(),
            curr: self.token.clone(),
            pos: self.scanner.get_position(),
            last_err: self.last_error_token.clone(),
        };
    }

    pub fn restore_at_mark(&mut self, mark: Mark) {
        self.prev_token = mark.prev;
        self.token = mark.curr;
        self.scanner.set_position(mark.pos);
        self.last_error_token = mark.last_err;
    }

    pub fn ttry<F>(&mut self, func: F) -> Option<NodeId>
    where
        F: Fn(&mut Self) -> Option<NodeId>,
    {
        let saved = self.mark();
        match func(self) {
            Some(n) => return Some(n),
            None => {
                self.restore_at_mark(saved);
                return None;
            }
        }
    }

    // =================
    // NODE HELPERS (creation, accession, mutation)
    // =================
    // _u methods do not check if the node_id exists in the tree

    pub fn create_node(&self, node_type: CssNodeType) -> CssNode {
        return CssNode::new(self.token.offset, self.token.length, node_type);
    }

    pub fn create<F, T>(&self, ctor: F) -> T
    where
        F: Fn(usize, usize) -> T,
    {
        return ctor(self.token.offset, self.token.length);
    }

    pub fn orphan(&mut self, css_node_type: CssNodeType) -> NodeId {
        return self.tree.0.orphan(self.create_node(css_node_type)).id();
    }

    pub fn node(&mut self, node_id: NodeId) -> Option<NodeRef<CssNode>> {
        return self.tree.0.get(node_id);
    }
    pub fn node_u(&mut self, node_id: NodeId) -> NodeRef<CssNode> {
        return unsafe { self.tree.0.get_unchecked(node_id) };
    }

    pub fn nodemut(&mut self, node_id: NodeId) -> Option<NodeMut<CssNode>> {
        return self.tree.0.get_mut(node_id);
    }
    pub fn nodemut_u(&mut self, node_id: NodeId) -> NodeMut<CssNode> {
        return unsafe { self.tree.0.get_unchecked_mut(node_id) };
    }

    pub fn value(&self, node_id: NodeId) -> Option<&CssNode> {
        return self.tree.0.get(node_id).map(|n| n.value());
    }
    pub fn value_u(&self, node_id: NodeId) -> &CssNode {
        return unsafe { self.tree.0.get_unchecked(node_id).value() };
    }

    // pub fn valuemut(&mut self, node_id: NodeId) -> Option<&mut CssNode> {
    //     return self.tree.0.get_mut(node_id).and_then(|mut nm| Some(nm.value()))
    // }
    // pub fn valuemut_u<'a>(&'a mut self, node_id: NodeId) -> &'a mut CssNode {
    //     return unsafe {self.tree.0.get_unchecked_mut(node_id).value()}
    // }

    pub fn append(&mut self, parent: NodeId, child: NodeId) -> bool {
        if let Some(mut p) = self.tree.0.get_mut(parent) {
            p.append_id(child);
            return true;
        }
        return false;
    }

    pub fn append_maybe(&mut self, parent: NodeId, child: Option<NodeId>) -> bool {
        if let Some(child) = child {
            self.append(parent, child);
            return true;
        }
        return false;
    }

    /// marks node with `node_id` with the specified error, performs the specified resync
    /// and
    pub fn finish_u(
        &mut self,
        node_id: NodeId,
        error: Option<ParseError>,
        resync_tokens: Option<&[TokenType]>,
        resync_stop_tokens: Option<&[TokenType]>,
    ) {
        if self.node_u(node_id).value().node_type != CssNodeType::Nodelist {
            if let Some(err) = error {
                self.mark_error_u(node_id, err, resync_tokens, resync_stop_tokens);
            }
            if let Some(prev) = &self.prev_token {
                // length with more tokens belonging together
                let prev_end = prev.offset + prev.length;
                let mut v = self.nodemut_u(node_id);
                v.value().length = if prev_end > v.value().offset {
                    // length from previous node end to `node_id` offset
                    prev_end - v.value().offset
                } else {
                    0
                };
            }
        }
    }

    pub fn varnish(&mut self, node_id: NodeId) -> Option<NodeId> {
        return self.finito_u(node_id, None, None, None);
    }

    pub fn finito_u(
        &mut self,
        node_id: NodeId,
        error: Option<ParseError>,
        resync_tokens: Option<&[TokenType]>,
        resync_stop_tokens: Option<&[TokenType]>,
    ) -> Option<NodeId> {
        self.finish_u(node_id, error, resync_tokens, resync_stop_tokens);
        return Some(node_id);
    }

    pub fn mark_error_u(
        &mut self,
        node_id: NodeId,
        error: ParseError,
        resync_tokens: Option<&[TokenType]>,
        resync_stop_tokens: Option<&[TokenType]>,
    ) {
        // vscode-css-languageservice has this check but it causes errors
        // here because last_error_token may be set in a parser combinator
        // branch that failed and was not attached to the tree
        // I think this works in vscode-css-languageservice because it is
        // a object reference compare
        if Some(&self.token) != self.last_error_token.as_ref() {
            // do not report twice on the same token
            let offset = self.token.offset;
            let length = self.token.length;
            self.nodemut_u(node_id).value().add_issue(Marker {
                error,
                level: Level::Error,
                message: "".to_string(),
                offset,
                length,
            });
            self.last_error_token = Some(self.token.clone());
        }
        if resync_tokens.is_some() || resync_stop_tokens.is_some() {
            self.resync(resync_tokens, resync_stop_tokens);
        }
    }

    // ===============
    // PEEK (inspectors)
    // ===============

    pub fn peek_ident(&self, text: &str) -> bool {
        return TokenType::Ident == self.token.token_type
            && text.len() == self.token.text.len()
            && text == self.token.text.to_lowercase();
    }

    pub fn peek_at_keyword(&self, text: &str) -> bool {
        return TokenType::AtKeyword == self.token.token_type
            && text.len() == self.token.text.len()
            && text == self.token.text.to_lowercase();
    }

    pub fn peek_delim(&self, text: &str) -> bool {
        return self.token.token_type == TokenType::Delim && text == self.token.text;
    }

    pub fn peek(&self, token_type: TokenType) -> bool {
        return self.token.token_type == token_type;
    }

    pub fn peek_one(&self, token_types: &[TokenType]) -> bool {
        return token_types.contains(&self.token.token_type);
    }

    /// Peeks regex in current token text, callers must anchor the regex match to
    /// the token text start with a `^`, if this is desired
    pub fn peek_regex(&self, token_type: TokenType, regex: Regex) -> bool {
        if self.token.token_type != token_type {
            return false;
        }
        return regex.is_match(&self.token.text);
    }

    pub fn has_whitespace(&self) -> bool {
        return self
            .prev_token
            .as_ref()
            .is_some_and(|pt| pt.offset + pt.length != self.token.offset);
    }

    // =================
    // CONSUME (mutators; may mutate self.token, self.prev_token, self.scanner)
    // =================

    pub fn consume_token(&mut self) {
        let t = std::mem::replace(&mut self.token, self.scanner.scan());
        self.prev_token = Some(t);
    }

    pub fn accept_unicode_range(&mut self) -> bool {
        if self.scanner.try_scan_unicode().is_some() {
            self.consume_token();
            return true;
        }
        return false;
    }

    pub fn accept_one_keyword(&mut self, keywords: &[&str]) -> bool {
        if self.token.token_type != TokenType::AtKeyword {
            return false;
        }
        for keyword in keywords {
            if keyword.len() == self.token.text.len() && keyword == &self.token.text.to_lowercase()
            {
                self.consume_token();
                return true;
            }
        }
        return false;
    }

    pub fn accept(&mut self, token_type: TokenType) -> bool {
        if token_type == self.token.token_type {
            self.consume_token();
            return true;
        }
        return false;
    }

    pub fn accept_ident(&mut self, text: &str) -> bool {
        if self.peek_ident(text) {
            self.consume_token();
            return true;
        }
        return false;
    }

    pub fn accept_at_keyword(&mut self, text: &str) -> bool {
        if self.peek_at_keyword(text) {
            self.consume_token();
            return true;
        }
        return false;
    }

    pub fn accept_delim(&mut self, text: &str) -> bool {
        if self.peek_delim(text) {
            self.consume_token();
            return true;
        }
        return false;
    }

    pub fn accept_unquoted_string(&mut self) -> bool {
        let pos = self.scanner.get_position();
        self.scanner.set_position(self.token.offset);
        if let Some(unquoted) = self.scanner.scan_unquoted_string() {
            self.token = unquoted;
            self.consume_token();
            return true;
        }
        self.scanner.set_position(pos);
        return false;
    }

    pub fn resync(
        &mut self,
        resync_tokens: Option<&[TokenType]>,
        resync_stop_tokens: Option<&[TokenType]>,
    ) -> bool {
        loop {
            if resync_tokens.is_some_and(|rts| rts.contains(&self.token.token_type)) {
                self.consume_token();
                return true;
            } else if resync_stop_tokens.is_some_and(|rts| rts.contains(&self.token.token_type)) {
                return true;
            } else if self.token.token_type == TokenType::EOF {
                return false;
            }
            self.token = self.scanner.scan();
        }
    }

    // ================
    // PARSE (mutators, may mutate any field of self)
    // ================

    #[tracing::instrument(skip_all)]
    pub fn create_stylesheet_with_string(&mut self, text: String) -> CssNodeTree {
        self.scanner.stream.set_source(text);
        self.token = self.scanner.scan();
        let node = self.parse_stylesheet();
        self.append(self.root, node);
        return CssNodeTree::new(self.take_tree(), self.take_source());
    }

    pub fn into_stylesheet(&mut self) -> CssNodeTree {
        let node = self.parse_stylesheet();
        self.append(self.root, node);
        return CssNodeTree::new(self.take_tree(), self.take_source());
    }

    pub fn into_css_node_tree(mut self) -> CssNodeTree {
        return CssNodeTree::new(self.take_tree(), self.take_source());
    }

    pub fn into_parsed_by_fn<F: FnMut(&mut Self) -> Option<NodeId>>(
        mut self,
        mut f: F,
    ) -> Option<CssNodeTree> {
        if let Some(id) = f(&mut self) {
            self.append(self.root, id);
            return Some(self.into_css_node_tree());
        }
        return None;
    }

    pub fn get_tree_parsed_by_fn<F: FnMut(&mut Self) -> Option<NodeId>>(
        &mut self,
        mut f: F,
    ) -> Option<&SourceLessCssNodeTree> {
        if let Some(id) = f(self) {
            self.append(self.root, id);
            return Some(&self.tree);
        }
        return None;
    }

    pub fn parse_node_by_fn<F: FnMut(&mut Self) -> Option<NodeId>>(
        &mut self,
        mut f: F,
    ) -> Option<NodeId> {
        return f(self)
    }

    pub fn parse_fn<F>(input: String, mut f: F) -> Option<CssNodeTree>
    where
        F: FnMut(&mut Self) -> Option<NodeId>,
    {
        let mut scanner = Scanner::default();
        scanner.set_source(input);
        let t = scanner.scan();
        let mut parser = Parser::new(scanner);
        parser.token = t;
        if f(&mut parser).is_some() {
            return Some(CssNodeTree::new(parser.take_tree(), parser.take_source()));
        }
        return None;
    }

    pub fn parse_stylesheet_fall(&mut self) -> Option<NodeId> {
        return Some(self.parse_stylesheet());
    }

    pub fn parse_stylesheet(&mut self) -> NodeId {
        let node = self.orphan(CssNodeType::Stylesheet);

        // Parse statements only valid at beginning of stylesheet
        while let Some(stsh_start) = self.parse_stylesheet_start() {
            self.append(node, stsh_start);
        }

        let mut in_recovery = false;
        let mut has_match;
        loop {
            loop {
                has_match = false;
                if let Some(statement) = self.parse_stylesheet_statement(false) {
                    self.append(node, statement);
                    has_match = true;
                    in_recovery = false;
                    if !self.peek(TokenType::EOF)
                        && Self::_needs_semicolon_after(self.value_u(statement))
                        && !self.accept(TokenType::SemiColon)
                    {
                        println!("bogobo 494");
                        self.mark_error_u(node, ParseError::SemiColonExpected, None, None);
                    }
                }
                while self.accept(TokenType::SemiColon)
                    || self.accept(TokenType::CDO)
                    || self.accept(TokenType::CDC)
                {
                    // accept empty statements
                    has_match = true;
                    in_recovery = false;
                }

                if !has_match {
                    break;
                }
            }
            if self.peek(TokenType::EOF) {
                break;
            }

            if !in_recovery {
                if self.peek(TokenType::AtKeyword) {
                    self.mark_error_u(node, ParseError::UnknownAtRule, None, None);
                } else {
                    self.mark_error_u(node, ParseError::RuleOrSelectorExpected, None, None);
                }
                in_recovery = true;
            }
            self.consume_token();

            if self.peek(TokenType::EOF) {
                break;
            }
        }
        self.varnish(node);
        return node;
    }

    pub fn parse_stylesheet_start(&mut self) -> Option<NodeId> {
        return self.parse_charset();
    }

    pub fn parse_stylesheet_statement(&mut self, is_nested: bool) -> Option<NodeId> {
        if self.peek(TokenType::AtKeyword) {
            return self.parse_stylesheet_at_statement(is_nested);
        }
        return self.parse_rule_set(is_nested);
    }

    pub fn parse_stylesheet_at_statement(&mut self, is_nested: bool) -> Option<NodeId> {
        return self
            .parse_import()
            .or_else(|| self.parse_media(is_nested))
            .or_else(|| self.parse_page())
            .or_else(|| self.parse_font_face())
            .or_else(|| self.parse_keyframe())
            .or_else(|| self.parse_supports(is_nested))
            .or_else(|| self.parse_layer(is_nested))
            .or_else(|| self.parse_property_at_rule())
            .or_else(|| self.parse_viewport())
            .or_else(|| self.parse_namespace())
            .or_else(|| self.parse_document())
            .or_else(|| self.parse_container())
            .or_else(|| self.parse_unknown_at_rule());
    }

    pub fn _tryparse_rule_set(&mut self, is_nested: bool) -> Option<NodeId> {
        let mark = self.mark();
        if self.parse_selector(is_nested).is_some() {
            while self.accept(TokenType::Comma) && self.parse_selector(is_nested).is_some() {
                // consume comma seperated selectors
            }
            if self.accept(TokenType::CurlyL) {
                self.restore_at_mark(mark);
                return self.parse_rule_set(is_nested);
            }
        }
        self.restore_at_mark(mark);
        return None;
    }

    pub fn parse_rule_set(&mut self, is_nested: bool) -> Option<NodeId> {
        let node = self.orphan(CssNodeType::_BodyDeclaration(BodyDeclaration {
            declarations: None,
            body_decl_type: BodyDeclarationType::RuleSet(RuleSet {
                selectors: self.root,
            }),
        }));

        let selectors = self.orphan(CssNodeType::Nodelist);

        if let Some(selector) = self.parse_selector(is_nested) {
            self.append(selectors, selector);
            self.nodemut_u(node)
                .value()
                .node_type
                .unchecked_rule_set()
                .selectors = selectors;
            //HOTFIX
            let selector_end = self.node_u(selector).value().end();
            let selectors_offset = self.node_u(selectors).value().offset;
            self.nodemut_u(selectors).value().length = selector_end - selectors_offset;
            // ENDHOTFIX
        } else {
            return None;
        }



        self.append(node, selectors);

        while self.accept(TokenType::Comma) {
            if let Some(selector) = self.parse_selector(is_nested) {
                self.append(selectors, selector);
                //HOTFIX TODO: VSCODE CSS LANGUAGESERVICE MAKES NODELIST.offset == NODELIST.length == -1
                let selector_end = self.node_u(selector).value().end();
                let selectors_offset = self.node_u(selectors).value().offset;
                self.nodemut_u(selectors).value().length = selector_end - selectors_offset;
                // ENDHOTFIX
            } else {
                self.finish_u(node, Some(ParseError::SelectorExpected), None, None);
                return Some(node);
            }
        }

        return self.parse_body(node, |s: &mut Self| s.parse_rule_set_declaration());
    }

    pub fn parse_rule_set_declaration_at_statement(&mut self) -> Option<NodeId> {
        return self
            .parse_media(true)
            .or_else(|| self.parse_supports(true))
            .or_else(|| self.parse_layer(true))
            .or_else(|| self.parse_unknown_at_rule());
    }

    pub fn parse_rule_set_declaration(&mut self) -> Option<NodeId> {
        // https://www.w3.org/TR/css-syntax-3/#consume-a-list-of-declarations
        if self.peek(TokenType::AtKeyword) {
            return self.parse_rule_set_declaration_at_statement();
        }
        if !self.peek(TokenType::Ident) {
            return self.parse_rule_set(true);
        }
        return self
            ._tryparse_rule_set(true)
            .or_else(|| self.parse_declaration(None));
    }

    pub fn _needs_semicolon_after(node: &CssNode) -> bool {
        // TODO: match exhaustively instead of with default
        match &node.node_type {
            CssNodeType::Namespace => false,
            CssNodeType::_BodyDeclaration(b) => match b.body_decl_type {
                BodyDeclarationType::Keyframe(..)
                | BodyDeclarationType::UnknownAtRule(..)
                | BodyDeclarationType::KeyframeSelector
                | BodyDeclarationType::ViewPort
                | BodyDeclarationType::Media
                | BodyDeclarationType::Page
                | BodyDeclarationType::PageBoxMarginBox
                | BodyDeclarationType::RuleSet(..)
                | BodyDeclarationType::IfStatement(..)
                | BodyDeclarationType::ForStatement(..)
                | BodyDeclarationType::EachStatement(..)
                | BodyDeclarationType::WhileStatement
                | BodyDeclarationType::MixinDeclaration(..)
                | BodyDeclarationType::FunctionDeclaration(..)
                | BodyDeclarationType::MixinContentDeclaration(..) => false,
                _ => false,
            },
            CssNodeType::ExtendsReference
            | CssNodeType::MixinContentReference
            | CssNodeType::ReturnStatement
            | CssNodeType::MediaQuery
            | CssNodeType::Debug
            | CssNodeType::Import
            | CssNodeType::AtApplyRule => true,
            CssNodeType::VariableDeclaration(v) => return v.needs_semicolon,
            CssNodeType::MixinReference(m) => return m.content.is_none(),
            CssNodeType::_AbstractDeclaration(a) => match &a.abstract_decl_type {
                AbstractDeclarationType::Declaration(d) => match d.declaration_type {
                    DeclarationType::CustomPropertyDeclaration(..) => return true,
                    DeclarationType::Declaration => return d.nested_properties.is_none(),
                },
                _ => false,
            },
            _ => false,
        }
    }

    /// `parse_declaration_func` must return Option<NodeId> of node of type `_AbstractDeclaration`
    pub fn parse_declarations<F>(&mut self, mut parse_declaration_func: F) -> Option<NodeId>
    where
        F: FnMut(&mut Self) -> Option<NodeId>,
    {
        let node = self
            .tree
            .0
            .orphan(self.create_node(CssNodeType::Declarations))
            .id();
        if !self.accept(TokenType::CurlyL) {
            return None;
        }

        let mut decl = parse_declaration_func(self);
        while let Some(d) = decl {
            self.append(node, d);
            if self.peek(TokenType::CurlyR) {
                break;
            }
            if Self::_needs_semicolon_after(self.value_u(d)) && !self.accept(TokenType::SemiColon) {
                println!("bogoba");
                self.finish_u(
                    node,
                    Some(ParseError::SemiColonExpected),
                    Some(&[TokenType::SemiColon, TokenType::CurlyR]),
                    None,
                );
                return Some(node);
            }
            // either decl doesn't need semicolon, or semicolon is was accepted, in which case we link it to the decl
            if let Some(prev) = self.prev_token.clone() {
                if prev.token_type == TokenType::SemiColon {
                    match self.nodemut_u(d).value().node_type {
                        CssNodeType::_AbstractDeclaration(ref mut a) => {
                            a.semicolon_position = prev.offset;
                        }
                        _ => {
                            // TODO vscode CSS language services doesn't seem to make sense as
                            // decl is cast to `Declaration` which is strange because `semicolon_position`
                            // is a field of `AbstractDeclaration` (superclass of declaration).
                            // This does not line up with the node types for which Self::needs_semicolon_after() is true.
                            // However, this method is called many times where `parse_declaration_func`
                            // is not a subclass of abstractdeclaration (e.g. through parse_body: parse_rule_set_declaration,
                            // _parse_keyframe_selector, parse_layer_declaration, parse_supports_declaration,
                            // parse_media_declaration, parse_page_declaration, parse_stylesheet_statement)

                            // panic!("internal code error: node should be Declaration")
                        }
                    }
                }
            }

            while self.accept(TokenType::SemiColon) {
                // accept empty statements
            }
            decl = parse_declaration_func(self);
        }
        if !self.accept(TokenType::CurlyR) {
            self.finish_u(
                node,
                Some(ParseError::RightCurlyExpected),
                Some(&[TokenType::CurlyR, TokenType::SemiColon]),
                None,
            );
            return Some(node);
        }
        return self.varnish(node);
    }

    // node.node_type.is_body_declaration() == true
    /// `parse_declaration_func` must return Option<NodeId> which has node type `_AbstractDeclaration`
    pub fn parse_body<F>(&mut self, node_id: NodeId, parse_declaration_func: F) -> Option<NodeId>
    where
        F: FnMut(&mut Self) -> Option<NodeId>,
    {
        #[cfg(debug_assertions)]
        match &self.value_u(node_id).node_type {
            CssNodeType::_BodyDeclaration(b) => {
                assert!(b.declarations.is_none(), "no good");
            }
            _ => {
                panic!("internal code error: parse_body(.., node: CssNode ..) node.node_type should be BodyDeclaration");
            }
        }

        if let Some(decl) = self.parse_declarations(parse_declaration_func) {
            self.nodemut_u(node_id)
                .value()
                .node_type
                .unchecked_body_decl()
                .declarations = Some(decl);
            self.append(node_id, decl);
        } else {
            return self.finito_u(
                node_id,
                Some(ParseError::LeftCurlyExpected),
                Some(&[TokenType::CurlyR, TokenType::SemiColon]),
                None,
            );
        }
        return self.varnish(node_id);
    }

    pub fn parse_selector(&mut self, is_nested: bool) -> Option<NodeId> {
        let node = self.orphan(CssNodeType::Selector);

        let mut has_content = false;
        if is_nested {
            // nested selectors can start with a combinator
            if let Some(comb) = self.parse_combinator() {
                self.append(node, comb);
                has_content = true;
            }
        }
        while let Some(simp) = self.parse_simple_selector() {
            self.append(node, simp);
            has_content = true;
            if let Some(comb) = self.parse_combinator() {
                self.append(node, comb);
            }
        }
        if has_content {
            return self.varnish(node);
        }
        return None;
    }

    pub fn parse_declaration(&mut self, stop_tokens: Option<&[TokenType]>) -> Option<NodeId> {
        if let Some(custom_prop_decl) = self._tryparse_custom_property_declaration(stop_tokens) {
            return Some(custom_prop_decl);
        }

        let node = self.orphan(CssNodeType::_AbstractDeclaration(AbstractDeclaration {
            colon_position: self.token.offset,
            semicolon_position: self.token.offset,
            abstract_decl_type: AbstractDeclarationType::Declaration(Declaration {
                property: self.root,
                expr: self.root,
                nested_properties: None,
                declaration_type: DeclarationType::Declaration,
            }),
        }));

        let Some(prop) = self.parse_property() else {
            return None;
        };
        self.append(node, prop);
        self.nodemut_u(node)
            .value()
            .node_type
            .unchecked_abst_decl_decl_decl_inner()
            .property = prop;

        if !self.accept(TokenType::Colon) {
            self.finish_u(
                node,
                Some(ParseError::ColonExpected),
                Some(&[TokenType::Colon]),
                Some(&[TokenType::SemiColon]),
            );
            return Some(node);
        }

        if let Some(prev) = &self.prev_token {
            self.nodemut_u(node)
                .value()
                .node_type
                .unchecked_abst_decl_inner()
                .colon_position = prev.offset;
        }

        if let Some(expr) = self.parse_expr(false) {
            self.append(node, expr);
            self.nodemut_u(node)
                .value()
                .node_type
                .unchecked_abst_decl_decl_decl_inner()
                .expr = expr;
        } else {
            self.finish_u(node, Some(ParseError::PropertyValueExpected), None, None);
            return Some(node);
        }

        if let Some(prio) = self.parse_prio() {
            self.append(node, prio);
        }

        if self.peek(TokenType::SemiColon) {
            self.nodemut_u(node)
                .value()
                .node_type
                .unchecked_abst_decl_inner()
                .semicolon_position = self.token.offset;
            // not part of the declaration, but useful information for code assist
        }

        return self.varnish(node);
    }

    pub fn _tryparse_custom_property_declaration(
        &mut self,
        stop_tokens: Option<&[TokenType]>,
    ) -> Option<NodeId> {
        if !self.peek_regex(TokenType::Ident, Regex::new("^--").unwrap()) {
            return None;
        }
        let node = self.orphan(CssNodeType::_AbstractDeclaration(AbstractDeclaration {
            colon_position: self.token.offset,
            semicolon_position: self.token.offset + self.token.length,
            abstract_decl_type: AbstractDeclarationType::Declaration(Declaration {
                property: self.root,
                expr: self.root,
                nested_properties: None,
                declaration_type: DeclarationType::CustomPropertyDeclaration(
                    CustomPropertyDeclaration {
                        property_set: self.root,
                    },
                ),
            }),
        }));

        macro_rules! get_abst_decl_mut {
            () => {
                self.nodemut_u(node)
                    .value()
                    .node_type
                    .unchecked_abst_decl_inner()
            };
        }
        macro_rules! get_decl_mut {
            () => {
                self.nodemut_u(node)
                    .value()
                    .node_type
                    .unchecked_abst_decl_decl_decl_inner()
            };
        }
        macro_rules! get_cust_prop_decl_mut {
            () => {
                self.nodemut_u(node)
                    .value()
                    .node_type
                    .unchecked_abst_decl_decl_custom_prop_decl_inner()
            };
        }

        // let mut get_abst_decl_mut = |s: &mut Parser| return ;
        // let mut get_decl_mut = |s: &mut Parser| return s.nodemut_u(node).value().node_type.unchecked_abst_decl_decl_decl_inner();
        // let mut get_cust_prop_decl_mut = |s: &mut Parser| return s.nodemut_u(node).value().node_type.unchecked_abst_decl_decl_custom_prop_decl_inner();

        if let Some(prop) = self.parse_property() {
            get_decl_mut!().property = prop;
            self.append(node, prop);
        } else {
            return None;
        }

        if !self.accept(TokenType::Colon) {
            self.finish_u(
                node,
                Some(ParseError::ColonExpected),
                Some(&[TokenType::Colon]),
                None,
            );
            return Some(node);
        }

        let mut set_colon_pos = false;
        if let Some(prev) = &self.prev_token {
            get_abst_decl_mut!().colon_position = prev.offset;
            set_colon_pos = true;
        }

        let mark = self.mark();

        // try to parse it as nested declaration
        if self.peek(TokenType::CurlyL) {
            let prop_set = self.orphan(CssNodeType::_BodyDeclaration(BodyDeclaration {
                declarations: None,
                body_decl_type: BodyDeclarationType::CustomPropertySet,
            }));
            let declarations =
                self.parse_declarations(|s: &mut Self| s.parse_rule_set_declaration());
            if let Some(decls) = declarations {
                self.nodemut_u(prop_set)
                    .value()
                    .node_type
                    .unchecked_body_decl()
                    .declarations = Some(decls);
                self.append(prop_set, decls);
                if !is_erroneous_recursive(self.node_u(decls)) {
                    if let Some(prio) = self.parse_prio() {
                        self.append(prop_set, prio);
                    }
                    if self.peek(TokenType::SemiColon) {
                        self.finish_u(prop_set, None, None, None);
                        get_cust_prop_decl_mut!().property_set = prop_set;
                        self.append(node, prop_set);
                        get_abst_decl_mut!().semicolon_position = self.token.offset; // not part of the declaration, but useful information for code assist
                        self.finish_u(node, None, None, None);
                        return Some(node);
                    }
                }
            }
            self.restore_at_mark(mark.clone());
        }

        // try to parse as expression
        if let Some(expr) = self.parse_expr(false) {
            if !is_erroneous_recursive(self.node_u(expr)) {
                self.parse_prio();
                let mut toks = vec![TokenType::SemiColon, TokenType::EOF];
                toks.append(&mut stop_tokens.unwrap_or(&[]).to_vec());
                if self.peek_one(&toks) {
                    get_decl_mut!().expr = expr;
                    self.append(node, expr);
                    if self.peek(TokenType::SemiColon) {
                        get_abst_decl_mut!().semicolon_position = self.token.offset;
                    }
                    return self.varnish(node);
                }
            }
        }
        // MARKERINO
        self.restore_at_mark(mark);
        let cust_prop_val =
            self.parse_custom_property_value(stop_tokens.unwrap_or(&[TokenType::CurlyR]));
        self.append(node, cust_prop_val);

        addchild!(prio);

        if set_colon_pos && self.token.offset == get_abst_decl_mut!().colon_position + 1 {
            return self.finito_u(node, Some(ParseError::PropertyValueExpected), None, None);
        } else {
            return self.varnish(node);
        }
    }

    /**
     * Parse custom property values.
     *
     * Based on https://www.w3.org/TR/css-variables/#syntax
     *
     * This code is somewhat unusual, as the allowed syntax is incredibly broad,
     * parsing almost any sequence of tokens, save for a small set of exceptions.
     * Unbalanced delimitors, invalid tokens, and declaration
     * terminators like semicolons and !important directives (when not inside
     * of delimitors).
     */
    pub fn parse_custom_property_value(&mut self, stop_tokens: &[TokenType]) -> NodeId {
        let node = self.orphan(CssNodeType::CustomPropertyValue);
        let mut curly_dep: i32 = 0;
        let mut paren_dep: i32 = 0;
        let mut brack_dep: i32 = 0;
        macro_rules! on_stop_token {
            () => {
                stop_tokens.contains(&self.token.token_type)
            };
        }
        macro_rules! is_top_lvl {
            () => {
                curly_dep == 0 && paren_dep == 0 && brack_dep == 0
            };
        }
        loop {
            match self.token.token_type.clone() {
                TokenType::SemiColon | TokenType::Exclamation => {
                    if is_top_lvl!() {
                        // exclamation or semicolon ends things if we are not inside delims
                        break;
                    }
                }
                TokenType::CurlyL => curly_dep += 1,
                TokenType::CurlyR => {
                    curly_dep -= 1;
                    if curly_dep < 0 {
                        // The property value has been terminated without a semicolon,
                        // and this is the last declaration in the ruleset
                        if on_stop_token!() && paren_dep == 0 && brack_dep == 0 {
                            break;
                        }
                        self.finito_u(node, Some(ParseError::LeftCurlyExpected), None, None);
                        return node;
                    }
                }
                TokenType::ParenthesisL => paren_dep += 1,
                TokenType::ParenthesisR => {
                    paren_dep -= 1;
                    if paren_dep < 0 {
                        if on_stop_token!() && brack_dep == 0 && curly_dep == 0 {
                            break;
                        }
                        self.finish_u(node, Some(ParseError::LeftParenthesisExpected), None, None);
                        return node;
                    }
                }
                TokenType::BracketL => brack_dep += 1,
                TokenType::BracketR => {
                    brack_dep -= 1;
                    if brack_dep < 0 {
                        self.finish_u(
                            node,
                            Some(ParseError::LeftSquareBracketExpected),
                            None,
                            None,
                        );
                        return node;
                    }
                }
                TokenType::BadString => break,
                TokenType::EOF => {
                    // we should not have reached the end of input,
                    // something is unterminated
                    let error = if brack_dep > 0 {
                        ParseError::RightSquareBracketExpected
                    } else if paren_dep > 0 {
                        ParseError::RightParenthesisExpected
                    } else {
                        ParseError::RightCurlyExpected
                    };
                    self.finish_u(node, Some(error), None, None);
                    return node;
                }
                _ => {
                    // Consume all the rest
                }
            }
            self.consume_token();
        }
        self.varnish(node);
        return node;
    }

    pub fn _tryparse_declaration(&mut self, stop_tokens: Option<&[TokenType]>) -> Option<NodeId> {
        let mark = self.mark();
        if self.parse_property().is_some() && self.accept(TokenType::Colon) {
            // looks like a declaration, go ahead
            self.restore_at_mark(mark);
            return self.parse_declaration(stop_tokens);
        }
        self.restore_at_mark(mark);
        return None;
    }

    pub fn parse_property(&mut self) -> Option<NodeId> {
        let node = self.orphan(CssNodeType::Property(Property {
            identifier: self.root,
        }));

        let mark = self.mark();
        if self.accept_delim("*") || self.accept_delim("_") {
            // support for IE 5.x. 6, and 7 hack: see http://en.wikipedia.org/wiki/CSS_filter#Star_hack
            if self.has_whitespace() {
                self.restore_at_mark(mark);
                return None;
            }
        }
        if let Some(prop) = self.parse_property_identifier() {
            self.nodemut_u(node)
                .value()
                .node_type
                .unchecked_inner_property()
                .identifier = prop;
            self.append(node, prop);
            return self.varnish(node);
        }
        return None;
    }

    pub fn parse_property_identifier(&mut self) -> Option<NodeId> {
        return self.parse_ident(None);
    }

    pub fn parse_charset(&mut self) -> Option<NodeId> {
        if !self.peek(TokenType::Charset) {
            return None;
        }
        let node = self.orphan(CssNodeType::Undefined); // TODO: why no node?

        self.consume_token(); // charset
        if !self.accept(TokenType::String) {
            println!("id88 1");
            self.finish_u(node, Some(ParseError::IdentifierExpected), None, None);
        }
        if !self.accept(TokenType::SemiColon) {
            println!("bogobee");
            self.finish_u(node, Some(ParseError::SemiColonExpected), None, None);
        }
        self.finish_u(node, None, None, None);

        return Some(node);
    }

    pub fn parse_import(&mut self) -> Option<NodeId> {
        // @import [ <url> | <string> ]
        //     [ layer | layer(<layer-name>) ]?
        //     <import-condition> ;

        // <import-conditions> = [ supports( [ <supports-condition> | <declaration> ] ) ]?
        //                      <media-query-list>?
        if !self.peek_at_keyword("@import") {
            return None;
        }

        let node = self.orphan(CssNodeType::Import);

        self.consume_token(); // @import

        if let Some(uri_lit) = self.parse_uri_literal() {
            self.append(node, uri_lit);
        } else if let Some(str_lit) = self.parse_string_literal() {
            self.append(node, str_lit);
        } else {
            return self.finito_u(node, Some(ParseError::URIOrStringExpected), None, None);
        }

        return self._completeparse_import(node);
    }

    pub fn _completeparse_import(&mut self, node: NodeId) -> Option<NodeId> {
        if self.accept_ident("layer") && self.accept(TokenType::ParenthesisL) {
            if let Some(layer_name) = self.parse_layer_name() {
                self.append(node, layer_name);
            } else {
                println!("id88 2");
                return self.finito_u(
                    node,
                    Some(ParseError::IdentifierExpected),
                    Some(&[TokenType::SemiColon]),
                    None,
                );
            }
            if !self.accept(TokenType::ParenthesisR) {
                return self.finito_u(
                    node,
                    Some(ParseError::RightParenthesisExpected),
                    Some(&[TokenType::ParenthesisR]),
                    None,
                );
            }
        }
        if self.accept_ident("supports") && self.accept(TokenType::ParenthesisL) {
            if let Some(decl_or_suppcond) = self
                ._tryparse_declaration(None)
                .or_else(|| self.parse_supports_condition())
            {
                self.append(node, decl_or_suppcond);
            }
            if !self.accept(TokenType::ParenthesisR) {
                return self.finito_u(
                    node,
                    Some(ParseError::RightParenthesisExpected),
                    Some(&[TokenType::ParenthesisR]),
                    None,
                );
            }
        }
        if !self.peek(TokenType::SemiColon) && !self.peek(TokenType::EOF) {
            if let Some(media_query_list) = self.parse_media_query_list() {
                self.append(node, media_query_list);
            }
        }
        self.finish_u(node, None, None, None);
        return Some(node);
    }

    pub fn parse_namespace(&mut self) -> Option<NodeId> {
        // http://www.w3.org/TR/css3-namespace/
        // namespace  : NAMESPACE_SYM S* [IDENT S*]? [STRING|URI] S* ';' S*
        if !self.peek_at_keyword("@namespace") {
            return None;
        }

        let node = self.orphan(CssNodeType::Namespace);
        self.consume_token(); // @namespace
        if let Some(uri_lit) = self.parse_uri_literal() {
            // uri literal also starts with ident
            self.append(node, uri_lit);
        } else {
            if let Some(ident) = self.parse_ident(None) {
                self.append(node, ident); // optional prefix
            }

            if let Some(uri_lit) = self.parse_uri_literal() {
                self.append(node, uri_lit);
            } else if let Some(str_lit) = self.parse_string_literal() {
                self.append(node, str_lit);
            } else {
                return self.finito_u(
                    node,
                    Some(ParseError::URIExpected),
                    Some(&[TokenType::SemiColon]),
                    None,
                ); // TODO: parserror should be URIorStringLiteralExpected?
            }
        }

        if !self.accept(TokenType::SemiColon) {
            return self.finito_u(node, Some(ParseError::SemiColonExpected), None, None);
        } else {
            return self.varnish(node);
        }
    }

    pub fn parse_font_face(&mut self) -> Option<NodeId> {
        if !self.peek_at_keyword("@font-face") {
            return None;
        }
        let node = self.orphan(CssNodeType::_BodyDeclaration(BodyDeclaration {
            declarations: None,
            body_decl_type: BodyDeclarationType::FontFace,
        }));

        self.consume_token(); // @font-face
        return self.parse_body(node, |s: &mut Self| s.parse_rule_set_declaration());
    }

    pub fn parse_viewport(&mut self) -> Option<NodeId> {
        if !self.peek_at_keyword("@-ms-viewport")
            && !self.peek_at_keyword("@-o-viewport")
            && !self.peek_at_keyword("@viewport")
        {
            return None;
        }
        let node = self.orphan(CssNodeType::_BodyDeclaration(BodyDeclaration {
            declarations: None,
            body_decl_type: BodyDeclarationType::ViewPort,
        }));
        self.consume_token(); // @..viewport
        return self.parse_body(node, |s: &mut Self| s.parse_rule_set_declaration());
    }

    pub fn parse_keyframe(&mut self) -> Option<NodeId> {
        if !self.peek_regex(
            TokenType::AtKeyword,
            RegexBuilder::new("^@(\\-(webkit|ms|moz|o)\\-)?keyframes$")
                .case_insensitive(true)
                .build()
                .unwrap(),
        ) {
            return None;
        }

        let node = self.orphan(CssNodeType::_BodyDeclaration(BodyDeclaration {
            declarations: None,
            body_decl_type: BodyDeclarationType::Keyframe(Keyframe {
                keyword: self.root,
                identifier: self.root,
            }),
        }));

        let at_node = self.orphan(CssNodeType::Undefined);
        self.consume_token(); // @keyframe
        self.finish_u(at_node, None, None, None);
        self.nodemut_u(node)
            .value()
            .node_type
            .unchecked_inner_keyword()
            .keyword = at_node;
        self.append(node, at_node);

        if self.tree.get_text(at_node, &self.scanner.stream.source) == "@-ms-keyframes" {
            // -ms-keyframes never existed
            self.mark_error_u(at_node, ParseError::UnknownKeyword, None, None);
        } // TODO: isn't get_text always 'unknown' because no text provider in at_node or node?

        if let Some(keyframe_ident) = self.parse_keyframe_ident() {
            self.nodemut_u(node)
                .value()
                .node_type
                .unchecked_inner_keyword()
                .identifier = keyframe_ident;
            self.append(node, keyframe_ident);
        } else {
            println!("id88 3");
            return self.finito_u(node, Some(ParseError::IdentifierExpected), None, None);
        }

        return self.parse_body(node, |s: &mut Self| s.parse_keyframe_selector());
    }

    pub fn parse_keyframe_ident(&mut self) -> Option<NodeId> {
        return self.parse_ident(Some(&[ReferenceType::Keyframe]));
    }

    pub fn parse_keyframe_selector(&mut self) -> Option<NodeId> {
        let node = self.orphan(CssNodeType::_BodyDeclaration(BodyDeclaration {
            declarations: None,
            body_decl_type: BodyDeclarationType::KeyframeSelector,
        }));

        let mut has_content = false;
        if addchildbool!(ident(None)) {
            has_content = true;
        }

        if self.accept(TokenType::Percentage) {
            has_content = true;
        }

        if !has_content {
            return None;
        }

        while self.accept(TokenType::Comma) {
            has_content = false;
            if addchildbool!(ident(None)) {
                has_content = true;
            }
            if self.accept(TokenType::Percentage) {
                has_content = true;
            }
            if !has_content {
                return self.finito_u(node, Some(ParseError::PercentageExpected), None, None);
            }
        }
        return self.parse_body(node, |s: &mut Self| s.parse_rule_set_declaration());
    }

    pub fn parse_tryparse_keyframe_selector(&mut self) -> Option<NodeId> {
        let node = self.orphan(CssNodeType::_BodyDeclaration(BodyDeclaration {
            declarations: None,
            body_decl_type: BodyDeclarationType::KeyframeSelector,
        }));

        let mark = self.mark();

        let mut has_content = false;
        if let Some(id) = self.parse_ident(None) {
            self.append(node, id);
            has_content = true;
        }
        if self.accept(TokenType::Percentage) {
            has_content = true;
        }
        if !has_content {
            return None;
        }
        while self.accept(TokenType::Comma) {
            has_content = false;
            if let Some(id) = self.parse_ident(None) {
                self.append(node, id);
                has_content = true;
            }
            if self.accept(TokenType::Percentage) {
                has_content = true;
            }
            if !has_content {
                self.restore_at_mark(mark);
                return None;
            }
        }

        if !self.peek(TokenType::CurlyL) {
            self.restore_at_mark(mark);
            return None;
        }

        return self.parse_body(node, |s: &mut Self| s.parse_rule_set_declaration());
    }

    pub fn parse_property_at_rule(&mut self) -> Option<NodeId> {
        // @property <custom-property-name> {
        // 	<declaration-list>
        //  }
        if !self.peek_at_keyword("@property") {
            return None;
        }
        let node = self.orphan(CssNodeType::_BodyDeclaration(BodyDeclaration {
            declarations: None,
            body_decl_type: BodyDeclarationType::PropertyAtRule(PropertyAtRule { name: self.root }),
        }));
        self.consume_token(); // @property
        if !self.peek_regex(TokenType::Ident, Regex::new("^--").unwrap()) {
            println!("id88 55");
            return self.finito_u(node, Some(ParseError::IdentifierExpected), None, None);
        }
        if let Some(id) = self.parse_ident(Some(&[ReferenceType::Property])) {
            self.append(node, id);
            self.nodemut_u(node)
                .value()
                .node_type
                .unchecked_property_at_rule()
                .name = id;
        } else {
            println!("id88 51");
            return self.finito_u(node, Some(ParseError::IdentifierExpected), None, None);
        }
        return self.parse_body(node, |s: &mut Self| s.parse_declaration(None));
    }

    pub fn parse_layer(&mut self, is_nested: bool) -> Option<NodeId> {
        // @layer layer-name {rules}
        // @layer layer-name;
        // @layer layer-name, layer-name, layer-name;
        // @layer {rules}
        if !self.peek_at_keyword("@layer") {
            return None;
        }
        let node = self.orphan(CssNodeType::_BodyDeclaration(BodyDeclaration {
            declarations: None,
            body_decl_type: BodyDeclarationType::Layer(Layer { names: self.root }),
        }));

        self.consume_token(); // @layer

        let names = self.parse_layer_namelist();
        if let Some(names) = names {
            self.append(node, names);
            self.nodemut_u(node)
                .value()
                .node_type
                .unchecked_layer()
                .names = names;
        }
        if (names.is_none() || self.node_u(names.unwrap()).children().count() == 1)
            && self.peek(TokenType::CurlyL)
        {
            return self.parse_body(node, |s: &mut Self| s.parse_layer_declaration(is_nested));
        }
        if !self.accept(TokenType::SemiColon) {
            return self.finito_u(node, Some(ParseError::SemiColonExpected), None, None);
        }
        return self.varnish(node);
    }

    pub fn parse_layer_declaration(&mut self, is_nested: bool) -> Option<NodeId> {
        if is_nested {
            // if nested, the body can contain rulesets, but also declarations
            return self
                ._tryparse_rule_set(true)
                .or_else(|| self._tryparse_declaration(None))
                .or_else(|| self.parse_stylesheet_statement(true));
        }
        return self.parse_stylesheet_statement(false);
    }

    pub fn parse_layer_namelist(&mut self) -> Option<NodeId> {
        let node = self.orphan(CssNodeType::LayerNameList);
        if let Some(layer_name) = self.parse_layer_name() {
            self.append(node, layer_name);
        } else {
            return None;
        }
        while self.accept(TokenType::Comma) {
            if !addchildbool!(layer_name) {
                println!("id88 11");
                return self.finito_u(node, Some(ParseError::IdentifierExpected), None, None);
            }
            // if let Some(layer_name) = self.parse_layer_name() {
            //     self.append(node, layer_name);
            // } else {
            //     return self.finito_u(node, Some(ParseError::IdentifierExpected), None, None);
            // }
        }
        return self.varnish(node);
    }

    pub fn parse_layer_name(&mut self) -> Option<NodeId> {
        // <layer-name> = <ident> [ '.' <ident> ]*
        let node = self.orphan(CssNodeType::LayerName);
        if let Some(id) = self.parse_ident(None) {
            self.append(node, id);
        } else {
            return None;
        }
        while !self.has_whitespace() && self.accept_delim(".") {
            if self.has_whitespace() || !addchildbool!(ident(None)) {
                println!("id88 9");
                return self.finito_u(node, Some(ParseError::IdentifierExpected), None, None);
            }
            // if !self.has_whitespace() {
            // } else if let Some(id) = self.parse_ident(None) {
            //     self.append(node, id);
            // } else {
            //     return self.finito_u(node, Some(ParseError::IdentifierExpected), None, None);
            // }
        }
        self.finish_u(node, None, None, None);
        return Some(node);
    }

    pub fn parse_supports(&mut self, is_nested: bool) -> Option<NodeId> {
        // SUPPORTS_SYM S* supports_condition '{' S* ruleset* '}' S*
        if !self.peek_at_keyword("@supports") {
            return None;
        }
        let node = self.orphan(CssNodeType::_BodyDeclaration(BodyDeclaration {
            declarations: None,
            body_decl_type: BodyDeclarationType::Supports,
        }));
        self.consume_token(); // @supports
        if let Some(supp_cond) = self.parse_supports_condition() {
            self.append(node, supp_cond);
        }
        return self.parse_body(node, |s: &mut Self| s.parse_supports_declaration(is_nested));
    }

    pub fn parse_supports_declaration(&mut self, is_nested: bool) -> Option<NodeId> {
        if is_nested {
            // if nested, the body can contain rulesets, but also declarations
            return self
                ._tryparse_rule_set(true)
                .or_else(|| self._tryparse_declaration(None))
                .or_else(|| self.parse_stylesheet_statement(true));
        }
        return self.parse_stylesheet_statement(false);
    }

    pub fn parse_supports_condition(&mut self) -> Option<NodeId> {
        // supports_condition : supports_negation | supports_conjunction | supports_disjunction | supports_condition_in_parens ;
        // supports_condition_in_parens: ( '(' S* supports_condition S* ')' ) | supports_declaration_condition | general_enclosed ;
        // supports_negation: NOT S+ supports_condition_in_parens ;
        // supports_conjunction: supports_condition_in_parens ( S+ AND S+ supports_condition_in_parens )+;
        // supports_disjunction: supports_condition_in_parens ( S+ OR S+ supports_condition_in_parens )+;
        // supports_declaration_condition: '(' S* declaration ')';
        // general_enclosed: ( FUNCTION | '(' ) ( any | unused )* ')' ;
        let node = self.orphan(CssNodeType::SupportsCondition(SupportsCondition {
            lef_parent: 0,
            rig_parent: 0,
        }));

        if self.accept_ident("not") {
            addchild!(supports_condition_in_parens);
        } else {
            addchild!(supports_condition_in_parens);
            if self.peek_regex(
                TokenType::Ident,
                RegexBuilder::new("^(and|or)$")
                    .case_insensitive(true)
                    .build()
                    .unwrap(),
            ) {
                let text = self.token.text.to_lowercase();
                while self.accept_ident(&text) {
                    if let Some(supp_cond_pare) = self.parse_supports_condition_in_parens() {
                        self.append(node, supp_cond_pare);
                    }
                }
            }
        }
        return self.varnish(node);
    }

    pub fn parse_supports_condition_in_parens(&mut self) -> Option<NodeId> {
        let node = self.orphan(CssNodeType::SupportsCondition(SupportsCondition {
            lef_parent: 0,
            rig_parent: 0,
        }));
        if self.accept(TokenType::ParenthesisL) {
            if let Some(prev) = &self.prev_token {
                self.nodemut_u(node)
                    .value()
                    .node_type
                    .unchecked_supports_condition()
                    .lef_parent = prev.offset;
            }
            if let Some(dec) = self._tryparse_declaration(Some(&[TokenType::ParenthesisR])) {
                self.append(node, dec);
            } else if self.parse_supports_condition().is_none() {
                return self.finito_u(node, Some(ParseError::ConditionExpected), None, None);
            }

            if !self.accept(TokenType::ParenthesisR) {
                return self.finito_u(
                    node,
                    Some(ParseError::RightParenthesisExpected),
                    Some(&[TokenType::ParenthesisR]),
                    None,
                );
            }
            if let Some(prev) = &self.prev_token {
                self.nodemut_u(node)
                    .value()
                    .node_type
                    .unchecked_supports_condition()
                    .rig_parent = prev.offset;
            }
            return self.varnish(node);
        } else if self.peek(TokenType::Ident) {
            let mark = self.mark();
            self.consume_token();
            if !self.has_whitespace() && self.accept(TokenType::ParenthesisL) {
                let mut open_parent_count = 1;
                while self.token.token_type != TokenType::EOF && open_parent_count != 0 {
                    if self.token.token_type == TokenType::ParenthesisL {
                        open_parent_count += 1;
                    } else if self.token.token_type == TokenType::ParenthesisR {
                        open_parent_count -= 1;
                    }
                    self.consume_token();
                }
                return self.varnish(node);
            } else {
                self.restore_at_mark(mark)
            }
        }
        return self.finito_u(
            node,
            Some(ParseError::LeftParenthesisExpected),
            Some(&[]),
            Some(&[TokenType::ParenthesisL]),
        );
    }

    pub fn parse_media_declaration(&mut self, is_nested: bool) -> Option<NodeId> {
        if is_nested {
            // if nested, the body can contain rulesets, but also declarations
            return self
                ._tryparse_rule_set(true)
                .or_else(|| self._tryparse_declaration(None))
                .or_else(|| self.parse_stylesheet_statement(true));
        }
        return self.parse_stylesheet_statement(false);
    }

    pub fn parse_media(&mut self, is_nested: bool) -> Option<NodeId> {
        // MEDIA_SYM S* media_query_list '{' S* ruleset* '}' S*
        // media_query_list : S* [media_query [ ',' S* media_query ]* ]?
        if !self.peek_at_keyword("@media") {
            return None;
        }
        let node = self.orphan(CssNodeType::_BodyDeclaration(BodyDeclaration {
            declarations: None,
            body_decl_type: BodyDeclarationType::Media,
        }));
        self.consume_token(); // @media
        if let Some(media_queries) = self.parse_media_query_list() {
            self.append(node, media_queries);
        } else {
            return self.finito_u(node, Some(ParseError::MediaQueryExpected), None, None);
        }
        return self.parse_body(node, |s: &mut Self| s.parse_media_declaration(is_nested));
    }

    pub fn parse_media_query_list(&mut self) -> Option<NodeId> {
        let node = self.orphan(CssNodeType::Medialist);
        if let Some(med_que) = self.parse_media_query() {
            self.append(node, med_que);
        } else {
            return self.finito_u(node, Some(ParseError::MediaQueryExpected), None, None);
        }
        while self.accept(TokenType::Comma) {
            if let Some(med_que) = self.parse_media_query() {
                self.append(node, med_que);
            } else {
                return self.finito_u(node, Some(ParseError::MediaQueryExpected), None, None);
            }
        }
        return self.varnish(node);
    }

    pub fn parse_media_query(&mut self) -> Option<NodeId> {
        // <media-query> = <media-condition> | [ not | only ]? <media-type> [ and <media-condition-without-or> ]?
        let node = self.orphan(CssNodeType::MediaQuery);
        let mark = self.mark();
        self.accept_ident("not");
        if !self.peek(TokenType::ParenthesisL) {
            if self.accept_ident("only") {
                // optional
            }
            if let Some(id) = self.parse_ident(None) {
                self.append(node, id);
            } else {
                return None;
            }
            if self.accept_ident("and") {
                if let Some(med_cond) = self.parse_media_condition() {
                    self.append(node, med_cond);
                }
            }
        } else {
            self.restore_at_mark(mark);
            if let Some(med_cond) = self.parse_media_condition() {
                self.append(node, med_cond);
            }
        }
        return self.varnish(node);
    }

    pub fn parse_ratio(&mut self) -> Option<NodeId> {
        let mark = self.mark();
        let node = self.orphan(CssNodeType::RatioValue);
        self.parse_numeric()?;
        if !self.accept_delim("/") {
            self.restore_at_mark(mark);
            return None;
        }
        if self.parse_numeric().is_none() {
            return self.finito_u(node, Some(ParseError::NumberExpected), None, None);
        }
        return self.varnish(node);
    }

    pub fn parse_media_condition(&mut self) -> Option<NodeId> {
        // <media-condition> = <media-not> | <media-and> | <media-or> | <media-in-parens>
        // <media-not> = not <media-in-parens>
        // <media-and> = <media-in-parens> [ and <media-in-parens> ]+
        // <media-or> = <media-in-parens> [ or <media-in-parens> ]+
        // <media-in-parens> = ( <media-condition> ) | <media-feature> | <general-enclosed>

        let node = self.orphan(CssNodeType::MediaCondition);
        self.accept_ident("not");
        let mut parse_expression = true;

        while parse_expression {
            if !self.accept(TokenType::ParenthesisL) {
                return self.finito_u(
                    node,
                    Some(ParseError::LeftParenthesisExpected),
                    None,
                    Some(&[TokenType::CurlyL]),
                );
            }
            if self.peek(TokenType::ParenthesisL) || self.peek_ident("not") {
                // <media-condition>
                if let Some(med_cond) = self.parse_media_condition() {
                    self.append(node, med_cond);
                }
            } else if let Some(med_feat) = self.parse_media_feature() {
                self.append(node, med_feat);
            }
            // not yet implemented: general enclosed    <TODO?>
            if !self.accept(TokenType::ParenthesisR) {
                return self.finito_u(node, Some(ParseError::RightParenthesisExpected), None, None);
            }
            parse_expression = self.accept_ident("and") || self.accept_ident("or");
        }
        return self.varnish(node);
    }

    pub fn parse_media_feature(&mut self) -> Option<NodeId> {
        let resync_stop_token: Option<&[TokenType]> = Some(&[TokenType::ParenthesisR]);

        let node = self.orphan(CssNodeType::MediaFeature);
        // <media-feature> = ( [ <mf-plain> | <mf-boolean> | <mf-range> ] )
        // <mf-plain> = <mf-name> : <mf-value>
        // <mf-boolean> = <mf-name>
        // <mf-range> = <mf-name> [ '<' | '>' ]? '='? <mf-value> | <mf-value> [ '<' | '>' ]? '='? <mf-name> | <mf-value> '<' '='? <mf-name> '<' '='? <mf-value> | <mf-value> '>' '='? <mf-name> '>' '='? <mf-value>

        if let Some(med_feat_name) = self.parse_media_feature_name() {
            self.append(node, med_feat_name);
            if self.accept(TokenType::Colon) {
                if let Some(_med_feat_val) = self.parse_media_feature_value() {
                    self.append(node, med_feat_name);
                } else {
                    return self.finito_u(
                        node,
                        Some(ParseError::TermExpected),
                        None,
                        resync_stop_token,
                    );
                }
            } else if self.parse_media_feature_range_operator() {
                if let Some(_med_feat_val) = self.parse_media_feature_value() {
                    self.append(node, med_feat_name);
                } else {
                    return self.finito_u(
                        node,
                        Some(ParseError::TermExpected),
                        None,
                        resync_stop_token,
                    );
                }
                if self.parse_media_feature_range_operator() {
                    if let Some(_med_feat_val) = self.parse_media_feature_value() {
                        self.append(node, med_feat_name);
                    } else {
                        return self.finito_u(
                            node,
                            Some(ParseError::TermExpected),
                            None,
                            resync_stop_token,
                        );
                    }
                }
            } else {
                // <mf-boolean> = <mf-name>
            }
        } else if let Some(med_feat_val) = self.parse_media_feature_value() {
            self.append(node, med_feat_val);
            if !self.parse_media_feature_range_operator() {
                return self.finito_u(
                    node,
                    Some(ParseError::OperatorExpected),
                    None,
                    resync_stop_token,
                );
            }
            if let Some(med_feat_name) = self.parse_media_feature_name() {
                self.append(node, med_feat_name);
            } else {
                println!("id88 41");
                return self.finito_u(
                    node,
                    Some(ParseError::IdentifierExpected),
                    None,
                    resync_stop_token,
                );
            }

            if self.parse_media_feature_range_operator() {
                if let Some(med_feat_val) = self.parse_media_feature_value() {
                    self.append(node, med_feat_val);
                } else {
                    return self.finito_u(
                        node,
                        Some(ParseError::TermExpected),
                        None,
                        resync_stop_token,
                    );
                }
            }
        } else {
            println!("id88 42");
            return self.finito_u(
                node,
                Some(ParseError::IdentifierExpected),
                None,
                resync_stop_token,
            );
        }
        return self.varnish(node);
    }

    pub fn parse_media_feature_range_operator(&mut self) -> bool {
        if self.accept_delim("<") || self.accept_delim(">") {
            if !self.has_whitespace() {
                self.accept_delim("=");
            }
            return true;
        } else if self.accept_delim("=") {
            return true;
        }
        return false;
    }

    pub fn parse_media_feature_name(&mut self) -> Option<NodeId> {
        return self.parse_ident(None);
    }

    pub fn parse_media_feature_value(&mut self) -> Option<NodeId> {
        return self.parse_ratio().or_else(|| self.parse_term_expression());
    }

    pub fn parse_medium(&mut self) -> Option<NodeId> {
        let node = self.orphan(CssNodeType::Undefined);
        if let Some(id) = self.parse_ident(None) {
            self.append(node, id);
            return self.varnish(node);
        }
        return None;
    }

    pub fn parse_page_declaration(&mut self) -> Option<NodeId> {
        return self
            .parse_page_margin_box()
            .or_else(|| self.parse_rule_set_declaration());
    }

    pub fn parse_page(&mut self) -> Option<NodeId> {
        // http://www.w3.org/TR/css3-page/
        // page_rule : PAGE_SYM S* page_selector_list '{' S* page_body '}' S*
        // page_body :  /* Can be empty */ declaration? [ ';' S* page_body ]? | page_margin_box page_body
        if !self.peek_at_keyword("@page") {
            return None;
        }
        let node = self.orphan(CssNodeType::_BodyDeclaration(BodyDeclaration {
            declarations: None,
            body_decl_type: BodyDeclarationType::Page,
        }));
        self.consume_token(); // @page
        if let Some(page_sel) = self.parse_page_selector() {
            self.append(node, page_sel);
            while self.accept(TokenType::Comma) {
                if let Some(page_sel) = self.parse_page_selector() {
                    self.append(node, page_sel);
                } else {
                    println!("id88 43");
                    return self.finito_u(node, Some(ParseError::IdentifierExpected), None, None);
                }
            }
        }
        return self.parse_body(node, |s: &mut Self| s.parse_page_declaration());
    }

    pub fn parse_page_margin_box(&mut self) -> Option<NodeId> {
        // page_margin_box :  margin_sym S* '{' S* declaration? [ ';' S* declaration? ]* '}' S*
        if !self.peek(TokenType::AtKeyword) {
            return None;
        }
        let node = self.orphan(CssNodeType::_BodyDeclaration(BodyDeclaration {
            declarations: None,
            body_decl_type: BodyDeclarationType::PageBoxMarginBox,
        }));
        if !self.accept_one_keyword(facts::PAGE_BOX_DIRECTIVES) {
            self.mark_error_u(
                node,
                ParseError::UnknownAtRule,
                Some(&[]),
                Some(&[TokenType::CurlyL]),
            );
        }
        return self.parse_body(node, |s: &mut Self| s.parse_rule_set_declaration());
    }

    pub fn parse_page_selector(&mut self) -> Option<NodeId> {
        // page_selector : pseudo_page+ | IDENT pseudo_page*
        // pseudo_page :  ':' [ "left" | "right" | "first" | "blank" ];
        if !self.peek(TokenType::Ident) && !self.peek(TokenType::Colon) {
            return None;
        }
        let node = self.orphan(CssNodeType::Undefined);
        if let Some(id) = self.parse_ident(None) {
            // optional ident
            self.append(node, id);
        }
        if self.accept(TokenType::Colon) {
            if let Some(id) = self.parse_ident(None) {
                self.append(node, id);
            } else {
                println!("id88 45");
                return self.finito_u(node, Some(ParseError::IdentifierExpected), None, None);
            }
        }
        return self.varnish(node);
    }

    pub fn parse_document(&mut self) -> Option<NodeId> {
        // -moz-document is experimental but has been pushed to css4
        if !self.peek_at_keyword("@-moz-document") {
            return None;
        }
        let node = self.orphan(CssNodeType::_BodyDeclaration(BodyDeclaration {
            declarations: None,
            body_decl_type: BodyDeclarationType::Document,
        }));
        self.consume_token(); // @-moz-document
        self.resync(Some(&[]), Some(&[TokenType::CurlyL])); // ignore all the rules
        return self.parse_body(node, |s: &mut Self| s.parse_stylesheet_statement(false));
    }

    pub fn parse_container(&mut self) -> Option<NodeId> {
        if !self.peek_at_keyword("@container") {
            return None;
        }
        let node = self.orphan(CssNodeType::_BodyDeclaration(BodyDeclaration {
            declarations: None,
            body_decl_type: BodyDeclarationType::Container,
        }));
        self.consume_token(); // @container
        if let Some(id) = self.parse_ident(None) {
            self.append(node, id); // optional container name
        }
        if let Some(cont_quer) = self.parse_container_query() {
            self.append(node, cont_quer); // optional TODO
        }

        return self.parse_body(node, |s: &mut Self| s.parse_stylesheet_statement(false));
    }

    pub fn parse_container_query(&mut self) -> Option<NodeId> {
        // <container-query>     = not <query-in-parens>
        //                         | <query-in-parens> [ [ and <query-in-parens> ]* | [ or <query-in-parens> ]* ]
        let node = self.orphan(CssNodeType::Undefined);
        if let Some(cont_quer) = self.parse_container_query_in_parens() {
            self.append(node, cont_quer);
        }
        if !self.accept_ident("not") {
            if self.peek_ident("and") {
                while self.accept_ident("and") {
                    if let Some(cont_quer) = self.parse_container_query_in_parens() {
                        self.append(node, cont_quer);
                    }
                }
            } else if self.peek_ident("or") {
                while self.accept_ident("or") {
                    if let Some(cont_quer) = self.parse_container_query_in_parens() {
                        self.append(node, cont_quer);
                    }
                }
            }
        }
        return self.varnish(node);
    }

    pub fn parse_container_query_in_parens(&mut self) -> Option<NodeId> {
        // <query-in-parens>     = ( <container-query> )
        // 					  | ( <size-feature> )
        // 					  | style( <style-query> )
        // 					  | <general-enclosed>
        let node = self.orphan(CssNodeType::Undefined);
        if self.accept(TokenType::ParenthesisL) {
            if self.peek_ident("not") || self.peek(TokenType::ParenthesisL) {
                if let Some(cont_quer) = self.parse_container_query() {
                    self.append(node, cont_quer);
                }
            } else if let Some(med_feat) = self.parse_media_feature() {
                self.append(node, med_feat);
            }
            if !self.accept(TokenType::ParenthesisR) {
                return self.finito_u(
                    node,
                    Some(ParseError::RightParenthesisExpected),
                    None,
                    Some(&[TokenType::CurlyL]),
                );
            }
        } else if self.accept_ident("style") {
            if self.has_whitespace() || !self.accept(TokenType::ParenthesisL) {
                return self.finito_u(
                    node,
                    Some(ParseError::LeftParenthesisExpected),
                    None,
                    Some(&[TokenType::CurlyL]),
                );
            }
            if let Some(style_que) = self.parse_style_query() {
                self.append(node, style_que);
            }
            if !self.accept(TokenType::ParenthesisR) {
                return self.finito_u(
                    node,
                    Some(ParseError::RightParenthesisExpected),
                    None,
                    Some(&[TokenType::CurlyL]),
                );
            }
        } else {
            return self.finito_u(
                node,
                Some(ParseError::LeftParenthesisExpected),
                None,
                Some(&[TokenType::CurlyL]),
            );
        }
        return self.varnish(node);
    }

    pub fn parse_style_query(&mut self) -> Option<NodeId> {
        // <style-query>         = not <style-in-parens>
        // 					  | <style-in-parens> [ [ and <style-in-parens> ]* | [ or <style-in-parens> ]* ]
        // 					  | <style-feature>
        // <style-in-parens>     = ( <style-query> )
        // 					  | ( <style-feature> )
        // 					  | <general-enclosed>
        let node = self.orphan(CssNodeType::Undefined);
        if self.accept_ident("not") {
            addchild!(style_in_parens);
        } else if self.peek(TokenType::ParenthesisL) {
            addchild!(style_in_parens);
            if self.peek_ident("and") {
                while self.accept_ident("and") {
                    addchild!(style_in_parens);
                }
            } else if self.peek_ident("or") {
                while self.accept_ident("or") {
                    addchild!(style_in_parens);
                }
            }
        } else {
            addchild!(declaration(Some(&[TokenType::ParenthesisR])))
        }
        return self.varnish(node);
    }

    pub fn parse_style_in_parens(&mut self) -> Option<NodeId> {
        let node = self.orphan(CssNodeType::Undefined);
        if self.accept(TokenType::ParenthesisL) {
            addchild!(style_query);
            if !self.accept(TokenType::ParenthesisR) {
                return self.finito_u(
                    node,
                    Some(ParseError::RightParenthesisExpected),
                    None,
                    Some(&[TokenType::CurlyL]),
                );
            }
        } else {
            return self.finito_u(
                node,
                Some(ParseError::LeftParenthesisExpected),
                None,
                Some(&[TokenType::CurlyL]),
            );
        }
        return self.varnish(node);
    }

    // https://www.w3.org/TR/css-syntax-3/#consume-an-at-rule
    pub fn parse_unknown_at_rule(&mut self) -> Option<NodeId> {
        if !self.peek(TokenType::AtKeyword) {
            return None;
        }
        let node = self.orphan(CssNodeType::_BodyDeclaration(BodyDeclaration {
            declarations: None,
            body_decl_type: BodyDeclarationType::UnknownAtRule(UnknownAtRule {
                at_rule_name: "".to_string(),
            }),
        }));
        addchild!(unknown_at_rule_name);

        let mut curly_l_count = 0;
        let mut curly_dep = 0;
        let mut parens_dep = 0;
        let mut bracks_dep = 0;
        macro_rules! is_top_lvl {
            () => {
                curly_dep == 0 && parens_dep == 0 && bracks_dep == 0
            };
        }
        use TokenType::*;
        loop {
            match self.token.token_type {
                SemiColon => {
                    if is_top_lvl!() {
                        break;
                    }
                }
                EOF => {
                    if curly_dep > 0 {
                        return self.finito_u(
                            node,
                            Some(ParseError::RightCurlyExpected),
                            None,
                            None,
                        );
                    } else if bracks_dep > 0 {
                        return self.finito_u(
                            node,
                            Some(ParseError::RightSquareBracketExpected),
                            None,
                            None,
                        );
                    } else if parens_dep > 0 {
                        return self.finito_u(
                            node,
                            Some(ParseError::RightParenthesisExpected),
                            None,
                            None,
                        );
                    } else {
                        return self.varnish(node);
                    }
                }
                CurlyL => {
                    curly_l_count += 1;
                    curly_dep += 1;
                }
                CurlyR => {
                    curly_dep -= 1;
                    // end of at-rule, consume curlyR and return node
                    if curly_l_count > 0 && curly_dep == 0 {
                        self.consume_token();
                        if bracks_dep > 0 {
                            return self.finito_u(
                                node,
                                Some(ParseError::RightSquareBracketExpected),
                                None,
                                None,
                            );
                        } else if parens_dep > 0 {
                            return self.finito_u(
                                node,
                                Some(ParseError::RightParenthesisExpected),
                                None,
                                None,
                            );
                        }
                        break;
                    }
                    if curly_dep < 0 {
                        // the property value has been terminated without a semicolon,
                        // and this is the last declaration in the ruleset
                        if parens_dep == 0 && bracks_dep == 0 {
                            break;
                        }
                        return self.finito_u(
                            node,
                            Some(ParseError::LeftCurlyExpected),
                            None,
                            None,
                        );
                    }
                }
                ParenthesisL => {
                    parens_dep += 1;
                }
                ParenthesisR => {
                    parens_dep -= 1;
                    if parens_dep < 0 {
                        return self.finito_u(
                            node,
                            Some(ParseError::LeftParenthesisExpected),
                            None,
                            None,
                        );
                    }
                }
                BracketL => {
                    bracks_dep += 1;
                }
                BracketR => {
                    bracks_dep -= 1;
                    if bracks_dep < 0 {
                        return self.finito_u(
                            node,
                            Some(ParseError::LeftSquareBracketExpected),
                            None,
                            None,
                        );
                    }
                }
                _ => {}
            }
            self.consume_token();
        }
        return Some(node);
    }

    pub fn parse_unknown_at_rule_name(&mut self) -> Option<NodeId> {
        let node = self.orphan(CssNodeType::Undefined);
        if self.accept(TokenType::AtKeyword) {
            return self.varnish(node);
        }
        return Some(node);
    }

    pub fn parse_operator(&mut self) -> Option<NodeId> {
        // these are operators for binary expressions
        if self.peek_delim("/")
            || self.peek_delim("*")
            || self.peek_delim("+")
            || self.peek_delim("-")
            || self.peek(TokenType::Dashmatch)
            || self.peek(TokenType::Includes)
            || self.peek(TokenType::SubstringOperator)
            || self.peek(TokenType::PrefixOperator)
            || self.peek(TokenType::SuffixOperator)
            || self.peek_delim("=")
        // does not stick to the standard here
        {
            let node = self.orphan(CssNodeType::Operator);
            self.consume_token();
            return self.varnish(node);
        } else {
            return None;
        }
    }

    pub fn parse_unary_operator(&mut self) -> Option<NodeId> {
        if !self.peek_delim("+") && !self.peek_delim("-") {
            return None;
        }
        let node = self.orphan(CssNodeType::Undefined);
        self.consume_token();
        return self.varnish(node);
    }

    pub fn parse_combinator(&mut self) -> Option<NodeId> {
        if self.peek_delim(">") {
            let node = self.orphan(CssNodeType::Undefined);
            self.consume_token();
            let mark = self.mark();
            if !self.has_whitespace() && self.accept_delim(">") {
                if !self.has_whitespace() && self.accept_delim(">") {
                    self.nodemut_u(node).value().node_type =
                        CssNodeType::SelectorCombinatorShadowPiercingDescendant;
                    return self.varnish(node);
                }
                self.restore_at_mark(mark);
            }
            self.nodemut_u(node).value().node_type = CssNodeType::SelectorCombinatorParent;
            return self.varnish(node);
        } else if self.peek_delim("+") {
            let node = self.orphan(CssNodeType::SelectorCombinatorSibling);
            self.consume_token();
            return self.varnish(node);
        } else if self.peek_delim("~") {
            let node = self.orphan(CssNodeType::SelectorCombinatorAllSiblings);
            self.consume_token();
            return self.varnish(node);
        } else if self.peek_delim("/") {
            let node = self.orphan(CssNodeType::SelectorCombinatorShadowPiercingDescendant);
            self.consume_token();
            let mark = self.mark();
            if !self.has_whitespace()
                && self.accept_ident("deep")
                && !self.has_whitespace()
                && self.accept_delim("/")
            {
                return self.varnish(node);
            }
            self.restore_at_mark(mark);
        }
        return None;
    }

    pub fn parse_simple_selector(&mut self) -> Option<NodeId> {
        // simple_selector
        //  : element_name [ HASH | class | attrib | pseudo ]* | [ HASH | class | attrib | pseudo ]+ ;
        let node = self.orphan(CssNodeType::SimpleSelector);
        let mut c = 0;
        if let Some(subby) = self
            .parse_element_name()
            .or_else(|| self.parse_nesting_selector())
        {
            self.append(node, subby);
            c += 1;
        }

        while (c == 0 || !self.has_whitespace()) && addchildbool!(simple_selector_body) {
            c += 1;
        }
        if c == 0 {
            return None;
        }
        return self.varnish(node);
    }

    pub fn parse_nesting_selector(&mut self) -> Option<NodeId> {
        if self.peek_delim("&") {
            let node = self.orphan(CssNodeType::SelectorCombinator);
            self.consume_token();
            return self.varnish(node);
        }
        return None;
    }

    pub fn parse_simple_selector_body(&mut self) -> Option<NodeId> {
        return self
            .parse_pseudo()
            .or_else(|| self.parse_hash())
            .or_else(|| self.parse_class())
            .or_else(|| self.parse_attribute());
    }

    pub fn parse_selector_ident(&mut self) -> Option<NodeId> {
        return self.parse_ident(None);
    }

    pub fn parse_hash(&mut self) -> Option<NodeId> {
        if !self.peek(TokenType::Hash) && !self.peek_delim("#") {
            return None;
        }
        let node = self.orphan(CssNodeType::IdentifierSelector);
        if self.accept_delim("#") {
            if self.has_whitespace() || !addchildbool!(selector_ident) {
                println!("id88 46");
                return self.finito_u(node, Some(ParseError::IdentifierExpected), None, None);
            }
        } else {
            self.consume_token(); // #
        }
        return self.varnish(node);
    }

    pub fn parse_class(&mut self) -> Option<NodeId> {
        // `.IDENT`
        if !self.peek_delim(".") {
            return None;
        }
        let node = self.orphan(CssNodeType::ClassSelector);
        self.consume_token(); // `.`
        if self.has_whitespace() || !addchildbool!(selector_ident) {
            println!("id88 47");
            return self.finito_u(node, Some(ParseError::IdentifierExpected), None, None);
        }
        return self.varnish(node);
    }

    pub fn parse_element_name(&mut self) -> Option<NodeId> {
        // (namespace? `|`)? IDENT | `*`
        let mark = self.mark();
        let node = self.orphan(CssNodeType::ElementNameSelector);
        addchild!(namespace_prefix);
        if !addchildbool!(selector_ident) && !self.accept_delim("*") {
            self.restore_at_mark(mark);
            return None;
        }
        return self.varnish(node);
    }

    pub fn parse_namespace_prefix(&mut self) -> Option<NodeId> {
        let mark = self.mark();
        let node = self.orphan(CssNodeType::NamespacePrefix);
        if !addchildbool!(ident(None)) && !self.accept_delim("*") {
            // namespace is optional
        }
        if !self.accept_delim("|") {
            self.restore_at_mark(mark);
            return None;
        }
        return self.varnish(node);
    }

    pub fn parse_attribute(&mut self) -> Option<NodeId> {
        // attrib : '[' S* IDENT S* [ [ '=' | INCLUDES | DASHMATCH ] S*   [ IDENT | STRING ] S* ]? ']'
        if !self.peek(TokenType::BracketL) {
            return None;
        }

        let node: NodeId = self.orphan(CssNodeType::AttributeSelector(AttributeSelector {
            namespace_prefix: None,
            operator: self.root,
            value: None,
            identifier: self.root,
        }));
        self.consume_token(); // `[`

        // optional attribute namespace
        addchild!(namespace_prefix then {
            self.nodemut_u(node).value().node_type.unchecked_attribute_selector().namespace_prefix = Some(namespace_prefix);
        });

        addchild!(ident (None) then {
            self.nodemut_u(node).value().node_type.unchecked_attribute_selector().identifier = ident;
        } else {
            println!("id88 48");
            return self.finito_u(node, Some(ParseError::IdentifierExpected), None, None)
        });

        addchild!(operator then {
            self.nodemut_u(node).value().node_type.unchecked_attribute_selector().operator = operator;
            addchild!(binary_expr then {
                self.nodemut_u(node).value().node_type.unchecked_attribute_selector().value = Some(binary_expr);
            });
            self.accept_ident("i"); // case insensitive matching
            self.accept_ident("s"); // case sensitive matching
        });

        if !self.accept(TokenType::BracketR) {
            return self.finito_u(
                node,
                Some(ParseError::RightSquareBracketExpected),
                None,
                None,
            );
        }
        return self.varnish(node);
    }

    pub fn parse_pseudo(&mut self) -> Option<NodeId> {
        // ':' [ IDENT | FUNCTION S* [IDENT S*]? ')' ]
        let Some(node) = self.try_parse_pseudo_identifier() else {
            return None;
        };
        if !self.has_whitespace() && self.accept(TokenType::ParenthesisL) {
            let try_as_selector = |s: &mut Self| {
                let selectors = s.orphan(CssNodeType::Undefined);
                if let Some(sel) = s.parse_selector(true) {
                    s.append(selectors, sel);
                } else {
                    return None;
                }
                while s.accept(TokenType::Comma) {
                    if let Some(_sel) = s.parse_selector(true) {
                        s.append(selectors, _sel);
                    } else {
                        break;
                    }
                }
                if s.peek(TokenType::ParenthesisR) {
                    return s.varnish(selectors);
                }
                return None;
            };

            let has_selector = if let Some(sel) = self.ttry(try_as_selector) {
                self.append(node, sel);
                true
            } else {
                false
            };

            if !has_selector && addchildbool!(binary_expr) && self.accept_ident("of") {
                if let Some(sel) = self.ttry(try_as_selector) {
                    self.append(node, sel);
                } else {
                    return self.finito_u(node, Some(ParseError::SelectorExpected), None, None);
                }
            }

            if !self.accept(TokenType::ParenthesisR) {
                return self.finito_u(node, Some(ParseError::RightParenthesisExpected), None, None);
            }
        }
        return self.varnish(node);
    }

    pub fn try_parse_pseudo_identifier(&mut self) -> Option<NodeId> {
        if !self.peek(TokenType::Colon) {
            return None;
        }
        let mark = self.mark();
        let node = self.orphan(CssNodeType::PseudoSelector);
        self.consume_token(); // ':'
        if self.has_whitespace() {
            self.restore_at_mark(mark);
            return None;
        }
        // optional, support ::
        self.accept(TokenType::Colon);
        if self.has_whitespace() || !addchildbool!(ident(None)) {
            return self.finito_u(node, Some(ParseError::IdentifierExpected), None, None);
        }
        return self.varnish(node);
    }

    pub fn _tryparse_prio(&mut self) -> Option<NodeId> {
        let mark = self.mark();
        if let Some(prio) = self.parse_prio() {
            return Some(prio);
        }
        self.restore_at_mark(mark);
        return None;
    }

    pub fn parse_prio(&mut self) -> Option<NodeId> {
        if !self.peek(TokenType::Exclamation) {
            return None;
        }
        let node = self.orphan(CssNodeType::Prio);
        if self.accept(TokenType::Exclamation) && self.accept_ident("important") {
            return self.varnish(node);
        }
        return None;
    }

    pub fn parse_expr(&mut self, stop_on_comma: bool) -> Option<NodeId> {
        let node = self.orphan(CssNodeType::Expression);
        if !addchildbool!(binary_expr) {
            return None;
        }
        loop {
            if self.peek(TokenType::Comma) {
                // optional
                if stop_on_comma {
                    return self.varnish(node);
                }
                self.consume_token();
            }
            if !addchildbool!(binary_expr) {
                break;
            }
        }
        return self.varnish(node);
    }

    pub fn parse_unicode_range(&mut self) -> Option<NodeId> {
        if !self.peek_ident("u") {
            return None;
        }
        let node = self.orphan(CssNodeType::UnicodeRange(UnicodeRange {
            range_start: self.root,
            range_end: self.root,
        }));
        if !self.accept_unicode_range() {
            return None;
        }
        return self.varnish(node);
    }

    pub fn parse_named_line(&mut self) -> Option<NodeId> {
        // https://www.w3.org/TR/css-grid-1/#named-lines
        if !self.peek(TokenType::BracketL) {
            return None;
        }
        let node = self.orphan(CssNodeType::GridLine);
        self.consume_token();
        while addchildbool!(ident(None)) {
            // loop
        }
        if !self.accept(TokenType::BracketR) {
            return self.finito_u(
                node,
                Some(ParseError::RightSquareBracketExpected),
                None,
                None,
            );
        }
        return self.varnish(node);
    }

    pub fn parse_binary_expr(&mut self) -> Option<NodeId> {
        return self.parse_binary_expr_internal(None, None);
    }

    pub fn parse_binary_expr_internal(
        &mut self,
        preparsed_left: Option<NodeId>,
        preparsed_oper: Option<NodeId>,
    ) -> Option<NodeId> {
        let mut node = self.orphan(CssNodeType::BinaryExpression(BinaryExpression {
            left: self.root,
            right: self.root,
            operator: self.root,
        }));
        if let Some(lef) = preparsed_left.or_else(|| self.parse_term()) {
            self.append(node, lef);
            self.nodemut_u(node)
                .value()
                .node_type
                .unchecked_binary_expr()
                .left = lef;
        } else {
            return None;
        }

        if let Some(oper) = preparsed_oper.or_else(|| self.parse_operator()) {
            self.append(node, oper);
            self.nodemut_u(node)
                .value()
                .node_type
                .unchecked_binary_expr()
                .operator = oper;
        } else {
            return self.varnish(node);
        }

        if let Some(term) = self.parse_term() {
            self.append(node, term);
            self.nodemut_u(node)
                .value()
                .node_type
                .unchecked_binary_expr()
                .right = term;
        } else {
            return self.finito_u(node, Some(ParseError::TermExpected), None, None);
        }

        // things needed for multiple binary expressions
        self.finish_u(node, None, None, None);
        if let Some(oper) = self.parse_operator() {
            if let Some(b) = self.parse_binary_expr_internal(Some(node), Some(oper)) {
                node = b;
            }
        }
        return self.varnish(node);
    }

    pub fn parse_term(&mut self) -> Option<NodeId> {
        let node = self.orphan(CssNodeType::Term(Term {
            operator: None,
            expression: self.root,
        }));

        // optional
        if let Some(uop) = self.parse_unary_operator() {
            self.append(node, uop);
            self.nodemut_u(node)
                .value()
                .node_type
                .unchecked_term()
                .operator = Some(uop);
        }

        if let Some(expr) = self.parse_term_expression() {
            self.append(node, expr);
            self.nodemut_u(node)
                .value()
                .node_type
                .unchecked_term()
                .expression = expr;
            return self.varnish(node);
        }
        return None;
    }

    pub fn parse_term_expression(&mut self) -> Option<NodeId> {
        return self
            .parse_uri_literal() // url before function
            .or_else(|| self.parse_unicode_range())
            .or_else(|| self.parse_function()) // function before ident
            .or_else(|| self.parse_ident(None))
            .or_else(|| self.parse_string_literal())
            .or_else(|| self.parse_numeric())
            .or_else(|| self.parse_hex_color())
            .or_else(|| self.parse_operation())
            .or_else(|| self.parse_named_line());
    }

    pub fn parse_operation(&mut self) -> Option<NodeId> {
        if !self.peek(TokenType::ParenthesisL) {
            return None;
        }
        let node = self.orphan(CssNodeType::Undefined);
        self.consume_token(); // '('
        addchild!(expr(false));
        if !self.accept(TokenType::ParenthesisR) {
            return self.finito_u(node, Some(ParseError::RightParenthesisExpected), None, None);
        }
        return self.varnish(node);
    }

    pub fn parse_numeric(&mut self) -> Option<NodeId> {
        if self.peek(TokenType::Num)
            || self.peek(TokenType::Percentage)
            || self.peek(TokenType::Resolution)
            || self.peek(TokenType::Length)
            || self.peek(TokenType::EMS)
            || self.peek(TokenType::EXS)
            || self.peek(TokenType::Angle)
            || self.peek(TokenType::Time)
            || self.peek(TokenType::Dimension)
            || self.peek(TokenType::ContainerQueryLength)
            || self.peek(TokenType::Freq)
        {
            let node = self.orphan(CssNodeType::NumericValue);
            self.consume_token();
            return self.varnish(node);
        }
        return None;
    }

    pub fn parse_string_literal(&mut self) -> Option<NodeId> {
        if !self.peek(TokenType::String) && !self.peek(TokenType::BadString) {
            return None;
        }
        let node = self.orphan(CssNodeType::StringLiteral);
        self.consume_token();
        return self.varnish(node);
    }

    pub fn parse_uri_literal(&mut self) -> Option<NodeId> {
        if !self.peek_regex(
            TokenType::Ident,
            RegexBuilder::new("^url(-prefix)?$")
                .case_insensitive(true)
                .build()
                .unwrap(),
        ) {
            return None;
        }
        let mark = self.mark();
        let node = self.orphan(CssNodeType::URILiteral);
        self.accept(TokenType::Ident);
        if self.has_whitespace() || !self.peek(TokenType::ParenthesisL) {
            self.restore_at_mark(mark);
            return None;
        }
        self.scanner.in_url = true;
        self.consume_token(); // '()'
        addchild!(url_argument); // optional
        self.scanner.in_url = false;
        if !self.accept(TokenType::ParenthesisR) {
            return self.finito_u(node, Some(ParseError::RightParenthesisExpected), None, None);
        }

        return self.varnish(node);
    }

    pub fn parse_url_argument(&mut self) -> Option<NodeId> {
        let node = self.orphan(CssNodeType::Undefined);
        if !self.accept(TokenType::String)
            && !self.accept(TokenType::BadString)
            && !self.accept_unquoted_string()
        {
            return None;
        }
        return self.varnish(node);
    }

    pub fn parse_ident(&mut self, reference_types: Option<&[ReferenceType]>) -> Option<NodeId> {
        if !self.peek(TokenType::Ident) {
            return None;
        }
        let node = self.orphan(CssNodeType::Identifier(Identifier {
            reference_types: reference_types.map(|r| r.to_vec()),
            is_custom_property: self.peek_regex(TokenType::Ident, Regex::new("^--").unwrap()),
        }));
        self.consume_token();
        return self.varnish(node);
    }

    pub fn parse_function(&mut self) -> Option<NodeId> {
        let mark = self.mark();

        let args = self.orphan(CssNodeType::Nodelist);

        let node = self.orphan(CssNodeType::_Invocation(Invocation {
            arguments: args,
            invocation_type: InvocationType::Function(Function {
                identifier: self.root,
            }),
        }));
        addchild!(function_identifier then {
            self.nodemut_u(node).value().node_type.unchecked_function().identifier = function_identifier;
        } else {
            return None
        });

        if self.has_whitespace() || !self.accept(TokenType::ParenthesisL) {
            self.restore_at_mark(mark);
            return None;
        }

        if let Some(fun_arg) = self.parse_function_argument() {
            self.append(args, fun_arg);
            while self.accept(TokenType::Comma) {
                if self.peek(TokenType::ParenthesisR) {
                    break;
                }
                if let Some(fun_arg) = self.parse_function_argument() {
                    self.append(args, fun_arg);
                } else {
                    self.mark_error_u(node, ParseError::ExpressionExpected, None, None);
                }
            }
        }

        if !self.accept(TokenType::ParenthesisR) {
            return self.finito_u(node, Some(ParseError::RightParenthesisExpected), None, None);
        }
        return self.varnish(node);
    }

    pub fn parse_function_identifier(&mut self) -> Option<NodeId> {
        if !self.peek(TokenType::Ident) {
            return None;
        }
        let node = self.orphan(CssNodeType::Identifier(Identifier {
            reference_types: Some(vec![ReferenceType::Function]),
            is_custom_property: false,
        }));

        if self.accept_ident("progid") {
            // support for IE7 specific filters: 'progid:DXImageTransform.Microsoft.MotionBlur(strength=13, direction=310)'
            if self.accept(TokenType::Colon) {
                while self.accept(TokenType::Ident) && self.accept_delim(".") {
                    // loop
                }
            }
            return self.varnish(node);
        }
        self.consume_token();
        return self.varnish(node);
    }

    pub fn parse_function_argument(&mut self) -> Option<NodeId> {
        let node = self.orphan(CssNodeType::FunctionArgument(FunctionArgument {
            identifier: None,
            value: self.root,
        }));
        addchild!(expr (true) then {
            self.nodemut_u(node).value().node_type.unchecked_function_argument().value = expr;
            return self.varnish(node)
        });
        return None;
    }

    pub fn parse_hex_color(&mut self) -> Option<NodeId> {
        if self.peek_regex(
            TokenType::Hash,
            Regex::new("^#[A-Fa-f0-9]{3}|[A-Fa-f0-9]{4}|[A-Fa-f0-9]{6}|[A-Fa-f0-9]{8}").unwrap(),
        ) {
            let node = self.orphan(CssNodeType::HexColorValue);
            self.consume_token();
            return self.varnish(node);
        }
        return None;
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new_with_text("".to_owned())
    }
}






#[cfg(test)]
mod test_css_parser {
    use super::*;
    use csslancer_macro::{assert_parse_error, assert_parse_node};

    fn assert_node<F: FnMut(&mut Parser) -> Option<NodeId>>(text: &str, f: F) -> CssNodeTree {
        let mut parser = Parser::new_with_text(text.to_owned());
        let tree = parser.get_tree_parsed_by_fn(f);
        assert!(tree.is_some(), "Failed parsing node");
        let tree = tree.unwrap();
        tree.assert_valid();
        // println!("{}", tree.fancy_string());
        let mut markers = Vec::new();
        tree.0.nodes()
            .filter(|n| tree.is_attached(n.id())) // discard orphans
            .for_each(|n| markers.append(&mut n.value().issues.clone()));

        assert!(
            markers.len() == 0,
            "node has errors: {}",
            markers
                .iter()
                .map(|m| &m.message)
                .fold("".to_owned(), |acc, nex| acc + "\n" + nex)
        );
        assert!(parser.accept(TokenType::EOF), "Expect scanner at EOF");
        return parser.into_css_node_tree();
    }

    fn assert_no_node<F: FnMut(&mut Parser) -> Option<NodeId>>(text: &str, f: F) {
        let mut parser = Parser::new_with_text(text.to_owned());
        let tree = parser.get_tree_parsed_by_fn(f);
        assert!(
            tree.is_none() || !parser.accept(TokenType::EOF),
            "Did not expect succesfully parsed node"
        );
    }

    fn assert_error<F: FnMut(&mut Parser) -> Option<NodeId>>(text: &str, f: F, error: ParseError) {
        let mut parser = Parser::new_with_text(text.to_owned());
        let tree = parser.get_tree_parsed_by_fn(f);
        assert!(tree.is_some(), "Failed parsing node");
        let tree = tree.unwrap();
        tree.assert_valid();
        let mut markers = Vec::new();
        tree.0.nodes()
            .filter(|n| tree.is_attached(n.id())) // discard orphans
            .for_each(|n| markers.append(&mut n.value().issues.clone()));

        assert!(
            markers.len() != 0,
            "node has NO errors, when they were expected"
        );
        markers.sort_by_key(|a| a.offset);
        assert_eq!(
            markers.first().unwrap().error,
            error,
            "incorrect error returned from parsing: {}",
            text
        )
    }

    #[test]
    fn stylesheet() {
        assert_parse_node!("@charset \"demo\" ;", stylesheet_fall);
        assert_parse_node!("body { margin: 0px; padding: 3em, 6em; }", stylesheet_fall);
        assert_parse_node!("--> <!--", stylesheet_fall);
        assert_parse_node!("", stylesheet_fall);
        assert_parse_node!("<!-- --> @import \"string\"; <!-- -->", stylesheet_fall);
        assert_parse_node!("@media asdsa { } <!-- --> <!-- -->", stylesheet_fall);
        assert_parse_node!("@media screen, projection { }", stylesheet_fall);
        assert_parse_node!(
            "@media screen and (max-width: 400px) {  @-ms-viewport { width: 320px; }}",
            stylesheet_fall
        );
        assert_parse_node!(
            "@-ms-viewport { width: 320px; height: 768px; }",
            stylesheet_fall
        );
        assert_parse_node!("#boo, far {} \n.far boo {}", stylesheet_fall);
        assert_parse_node!("@-moz-keyframes darkWordHighlight { from { background-color: inherit; } to { background-color: rgba(83, 83, 83, 0.7); } }", stylesheet_fall);
        assert_parse_node!("@page { margin: 2.5cm; }", stylesheet_fall);
        assert_parse_node!(
            "@font-face { font-family: \"Example Font\"; }",
            stylesheet_fall
        );
        assert_parse_node!(
            "@namespace \"http://www.w3.org/1999/xhtml\";",
            stylesheet_fall
        );
        assert_parse_node!("@namespace pref url(http://test);", stylesheet_fall);
        assert_parse_node!("@-moz-document url(http://test), url-prefix(http://www.w3.org/Style/) { body { color: purple; background: yellow; } }", stylesheet_fall);
        assert_parse_node!("E E[foo] E[foo=\"bar\"] E[foo~=\"bar\"] E[foo^=\"bar\"] E[foo$=\"bar\"] E[foo*=\"bar\"] E[foo|=\"en\"] {}", stylesheet_fall);
        assert_parse_node!("input[type=\"submit\"] {}", stylesheet_fall);
        assert_parse_node!("E:root E:nth-child(n) E:nth-last-child(n) E:nth-of-type(n) E:nth-last-of-type(n) E:first-child E:last-child {}", stylesheet_fall);
        assert_parse_node!("E:first-of-type E:last-of-type E:only-child E:only-of-type E:empty E:link E:visited E:active E:hover E:focus E:target E:lang(fr) E:enabled E:disabled E:checked {}", stylesheet_fall);
        assert_parse_node!(
            "E::first-line E::first-letter E::before E::after {}",
            stylesheet_fall
        );
        assert_parse_node!("E.warning E#myid E:not(s) {}", stylesheet_fall);
        assert_parse_error!("@namespace;", stylesheet_fall, URIExpected);
        assert_parse_error!(
            "@namespace url(http://test)",
            stylesheet_fall,
            SemiColonExpected
        );
        assert_parse_error!("@charset;", stylesheet_fall, IdentifierExpected);
        assert_parse_error!("@charset 'utf8'", stylesheet_fall, SemiColonExpected);
    }

    #[test]
    fn stylesheet_graceful_unknown_rules() {
        assert_parse_node!("@unknown-rule;", stylesheet_fall);
        assert_parse_node!("@unknown-rule 'foo';", stylesheet_fall);
        assert_parse_node!("@unknown-rule (foo) {}", stylesheet_fall);
        assert_parse_node!("@unknown-rule (foo) { .bar {} }", stylesheet_fall);
        assert_parse_node!("@mskeyframes darkWordHighlight { from { background-color: inherit; } to { background-color: rgba(83, 83, 83, 0.7); } }", stylesheet_fall);
        assert_parse_node!("foo { @unknown-rule; }", stylesheet_fall);

        assert_parse_error!(
            "@unknown-rule (;",
            stylesheet_fall,
            RightParenthesisExpected
        );
        assert_parse_error!(
            "@unknown-rule [foo",
            stylesheet_fall,
            RightSquareBracketExpected
        );
        assert_parse_error!(
            "@unknown-rule { [foo }",
            stylesheet_fall,
            RightSquareBracketExpected
        );
        assert_parse_error!("@unknown-rule (foo) {", stylesheet_fall, RightCurlyExpected);
        assert_parse_error!(
            "@unknown-rule (foo) { .bar {}",
            stylesheet_fall,
            RightCurlyExpected
        );
    }

    #[test]
    fn stylesheet_unknown_rules_node_proper_end() {
        // Microsoft/vscode#53159
        let tree = assert_parse_node!("@unknown-rule (foo) {} .foo {}", stylesheet_fall);

        let unknown_at_rule = tree
            .0
             .0
            .root()
            .first_child()
            .unwrap()
            .first_child()
            .unwrap();
        assert!(unknown_at_rule
            .value()
            .node_type
            .same_node_type(&CssNodeType::_BodyDeclaration(BodyDeclaration {
                declarations: None,
                body_decl_type: BodyDeclarationType::UnknownAtRule(UnknownAtRule {
                    at_rule_name: String::new()
                })
            })));
        assert_eq!(unknown_at_rule.value().offset, 0);
        assert_eq!(unknown_at_rule.value().length, 13);

        // microsoft/vscode-css-languageservice#237
        assert_parse_node!(
            ".foo { @apply p-4 bg-neutral-50; min-height: var(--space-14); }",
            stylesheet_fall
        );
    }

    #[test]
    fn stylesheet_panic() {
        assert_parse_error!(
            "#boo, far } \n.far boo {}",
            stylesheet_fall,
            LeftCurlyExpected
        );
        assert_parse_error!(
            "#boo, far { far: 43px; \n.far boo {}",
            stylesheet_fall,
            RightCurlyExpected
        );
        assert_parse_error!(
            "- @import \"foo\";",
            stylesheet_fall,
            RuleOrSelectorExpected
        );
    }

    #[test]
    fn font_face() {
        assert_parse_node!("@font-face {}", font_face);
        assert_parse_node!("@font-face { src: url(http://test) }", font_face);
        assert_parse_node!(
            "@font-face { font-style: normal; font-stretch: normal; }",
            font_face
        );
        assert_parse_node!("@font-face { unicode-range: U+0021-007F }", font_face);
        assert_parse_error!(
            "@font-face { font-style: normal font-stretch: normal; }",
            font_face,
            SemiColonExpected
        );
    }

    #[test]
    fn keyframe_selector() {
        assert_parse_node!("from {}", keyframe_selector);
        assert_parse_node!("to {}", keyframe_selector);
        assert_parse_node!("0% {}", keyframe_selector);
        assert_parse_node!("10% {}", keyframe_selector);
        assert_parse_node!("cover 10% {}", keyframe_selector);
        assert_parse_node!("100000% {}", keyframe_selector);
        assert_parse_node!("from { width: 100% }", keyframe_selector);
        assert_parse_node!("from { width: 100%; to: 10px; }", keyframe_selector);
        assert_parse_node!("from, to { width: 10px; }", keyframe_selector);
        assert_parse_node!("10%, to { width: 10px; }", keyframe_selector);
        assert_parse_node!("from, 20% { width: 10px; }", keyframe_selector);
        assert_parse_node!("10%, 20% { width: 10px; }", keyframe_selector);
        assert_parse_node!("cover 10% {}", keyframe_selector);
        assert_parse_node!("cover 10%, exit 20% {}", keyframe_selector);
        assert_parse_node!("10%, exit 20% {}", keyframe_selector);
        assert_parse_node!("from, exit 20% {}", keyframe_selector);
        assert_parse_node!("cover 10%, to {}", keyframe_selector);
        assert_parse_node!("cover 10%, 20% {}", keyframe_selector);
    }

    #[test]
    fn at_keyframe() {
        assert_parse_node!("@keyframes name {}", keyframe);
        assert_parse_node!("@-webkit-keyframes name {}", keyframe);
        assert_parse_node!("@-o-keyframes name {}", keyframe);
        assert_parse_node!("@-moz-keyframes name {}", keyframe);
        assert_parse_node!("@keyframes name { from {} to {}}", keyframe);
        assert_parse_node!("@keyframes name { from {} 80% {} 100% {}}", keyframe);
        assert_parse_node!(
            "@keyframes name { from { top: 0px; } 80% { top: 100px; } 100% { top: 50px; }}",
            keyframe
        );
        assert_parse_node!(
            "@keyframes name { from { top: 0px; } 70%, 80% { top: 100px; } 100% { top: 50px; }}",
            keyframe
        );
        assert_parse_node!(
            "@keyframes name { from { top: 0px; left: 1px; right: 2px }}",
            keyframe
        );
        assert_parse_node!(
            "@keyframes name { exit 50% { top: 0px; left: 1px; right: 2px }}",
            keyframe
        );
        assert_parse_error!(
            "@keyframes name { from { top: 0px; left: 1px, right: 2px }}",
            keyframe,
            SemiColonExpected
        );
        assert_parse_error!("@keyframes )", keyframe, IdentifierExpected);
        assert_parse_error!(
            "@keyframes name { { top: 0px; } }",
            keyframe,
            RightCurlyExpected
        );
        assert_parse_error!("@keyframes name { from, #123", keyframe, PercentageExpected);
        assert_parse_error!(
            "@keyframes name { 10% from { top: 0px; } }",
            keyframe,
            LeftCurlyExpected
        );
        assert_parse_error!(
            "@keyframes name { 10% 20% { top: 0px; } }",
            keyframe,
            LeftCurlyExpected
        );
        assert_parse_error!(
            "@keyframes name { from to { top: 0px; } }",
            keyframe,
            LeftCurlyExpected
        );
    }

    #[test]
    fn at_property() {
        assert_parse_node!(
            "@property --my-color { syntax: '<color>'; inherits: false; initial-value: #c0ffee; }",
            stylesheet_fall
        );
        assert_parse_error!("@property  {  }", stylesheet_fall, IdentifierExpected);
    }

    #[test]
    fn at_container() {
        assert_parse_node!(
            "@container (width <= 150px) { #inner { background-color: skyblue; }}",
            stylesheet_fall
        );
        assert_parse_node!(
            "@container card (inline-size > 30em) and style(--responsive: true) { }",
            stylesheet_fall
        );
        assert_parse_node!(
            "@container card (inline-size > 30em) { @container style(--responsive: true) {} }",
            stylesheet_fall
        );
    }

    #[test]
    fn at_container_query_len_units() {
        assert_parse_node!(
            "@container (min-width: 700px) { .card h2 { font-size: max(1.5em, 1.23em + 2cqi); } }",
            stylesheet_fall
        );
    }

    #[test]
    fn at_import() {
        assert_parse_node!("@import \"asdasdsa\"", import);
        assert_parse_node!("@ImPort \"asdsadsa\"", import);
        assert_parse_node!("@import \"asdasd\" dsfsdf", import);
        assert_parse_node!("@import \"foo\";", stylesheet_fall);
        assert_parse_node!(
            "@import url(/css/screen.css) screen, projection;",
            stylesheet_fall
        );
        assert_parse_node!(
            "@import url('landscape.css') screen and (orientation:landscape);",
            stylesheet_fall
        );
        assert_parse_node!(
            "@import url(\"/inc/Styles/full.css\") (min-width: 940px);",
            stylesheet_fall
        );
        assert_parse_node!(
            "@import url(style.css) screen and (min-width:600px);",
            stylesheet_fall
        );
        assert_parse_node!(
            "@import url(\"./700.css\") only screen and (max-width: 700px);",
            stylesheet_fall
        );

        assert_parse_node!("@import url(\"override.css\") layer;", stylesheet_fall);
        assert_parse_node!(
            "@import url(\"tabs.css\") layer(framework.component);",
            stylesheet_fall
        );

        assert_parse_node!(
            "@import \"mystyle.css\" supports(display: flex);",
            stylesheet_fall
        );

        assert_parse_node!(
            "@import url(\"narrow.css\") supports(display: flex) handheld and (max-width: 400px);",
            stylesheet_fall
        );
        assert_parse_node!(
            "@import url(\"fallback-layout.css\") supports(not (display: flex));",
            stylesheet_fall
        );

        assert_parse_error!("@import", stylesheet_fall, URIOrStringExpected);
    }

    #[test]
    fn at_supports() {
        assert_parse_node!(
            "@supports ( display: flexbox ) { body { display: flexbox } }",
            supports(false)
        );
        assert_parse_node!("@supports not (display: flexbox) { .outline { box-shadow: 2px 2px 2px black; /* unprefixed last */ } }", supports(false));
        assert_parse_node!("@supports ( box-shadow: 2px 2px 2px black ) or ( -moz-box-shadow: 2px 2px 2px black ) or ( -webkit-box-shadow: 2px 2px 2px black ) { }", supports(false));
        assert_parse_node!("@supports ((transition-property: color) or (animation-name: foo)) and (transform: rotate(10deg)) { }", supports(false));
        assert_parse_node!("@supports ((display: flexbox)) { }", supports(false));
        assert_parse_node!(
            "@supports (display: flexbox !important) { }",
            supports(false)
        );
        assert_parse_node!(
            "@supports (grid-area: auto) { @media screen and (min-width: 768px) { .me { } } }",
            supports(false)
        );
        assert_parse_node!("@supports (column-width: 1rem) OR (-moz-column-width: 1rem) OR (-webkit-column-width: 1rem) oR (-x-column-width: 1rem) { }", supports(false)); // #49288
        assert_parse_node!("@supports not (--validValue: , 0 ) {}", supports(false)); // #82178
        assert_parse_error!("@supports (transition-property: color) or (animation-name: foo) and (transform: rotate(10deg)) { }", supports(false), LeftCurlyExpected);
        assert_parse_error!(
            "@supports display: flexbox { }",
            supports(false),
            LeftParenthesisExpected
        );
    }

    #[test]
    fn at_media() {
        assert_parse_node!("@media asdsa { }", media(false));
        assert_parse_node!("@meDia sadd{}  ", media(false));
        assert_parse_node!("@media somename, othername2 { }", media(false));
        assert_parse_node!("@media only screen and (max-width:850px) { }", media(false));
        assert_parse_node!("@media only screen and (max-width:850px) { }", media(false));
        assert_parse_node!("@media all and (min-width:500px) { }", media(false));
        assert_parse_node!(
            "@media screen and (color), projection and (color) { }",
            media(false)
        );
        assert_parse_node!(
            "@media not screen and (device-aspect-ratio: 16/9) { }",
            media(false)
        );
        assert_parse_node!(
            "@media print and (min-resolution: 300dpi) { }",
            media(false)
        );
        assert_parse_node!(
            "@media print and (min-resolution: 118dpcm) { }",
            media(false)
        );
        assert_parse_node!(
            "@media print { @page { margin: 10% } blockquote, pre { page-break-inside: avoid } }",
            media(false)
        );
        assert_parse_node!("@media print { body:before { } }", media(false));
        assert_parse_node!(
            "@media not (-moz-os-version: windows-win7) { }",
            media(false)
        );
        assert_parse_node!(
            "@media not (not (-moz-os-version: windows-win7)) { }",
            media(false)
        );
        assert_parse_node!("@media (height > 600px) { }", media(false));
        assert_parse_node!("@media (height < 600px) { }", media(false));
        assert_parse_node!("@media (height <= 600px) { }", media(false));
        assert_parse_node!("@media (400px <= width <= 700px) { }", media(false));
        assert_parse_node!("@media (400px >= width >= 700px) { }", media(false));
        assert_parse_node!(
            "@media screen and (750px <= width < 900px) { }",
            media(false)
        );
        assert_parse_error!(
            "@media somename othername2 { }",
            media(false),
            LeftCurlyExpected
        );
        assert_parse_error!("@media not, screen { }", media(false), MediaQueryExpected);
        assert_parse_error!(
            "@media not screen and foo { }",
            media(false),
            LeftParenthesisExpected
        );
        assert_parse_error!(
            "@media not screen and () { }",
            media(false),
            IdentifierExpected
        );
        assert_parse_error!(
            "@media not screen and (color:) { }",
            media(false),
            TermExpected
        );
        assert_parse_error!(
            "@media not screen and (color:#234567 { }",
            media(false),
            RightParenthesisExpected
        );
    }

    #[test]
    fn media_list() {
        assert_parse_node!("somename", media_query_list);
        assert_parse_node!("somename, othername", media_query_list);
        assert_parse_node!("not all and (monochrome)", media_query_list);
    }

    #[test]
    fn medium() {
        assert_parse_node!("somename", medium);
        assert_parse_node!("-asdas", medium);
        assert_parse_node!("-asda34s", medium);
    }

    #[test]
    fn at_page() {
        assert_parse_node!("@page : name{ }", page);
        assert_parse_node!("@page :left, :right { }", page);
        assert_parse_node!("@page : name{ some : \"asdas\" }", page);
        assert_parse_node!("@page : name{ some : \"asdas\" !important }", page);
        assert_parse_node!(
            "@page : name{ some : \"asdas\" !important; some : \"asdas\" !important }",
            page
        );
        assert_parse_node!("@page rotated { size : landscape }", page);
        assert_parse_node!("@page :left { margin-left: 4cm; margin-right: 3cm; }", page);
        assert_parse_node!(
            "@page {  @top-right-corner { content: url(foo.png); border: solid green; } }",
            page
        );
        assert_parse_node!("@page {  @top-left-corner { content: \" \"; border: solid green; } @bottom-right-corner { content: counter(page); border: solid green; } }", page);
        assert_parse_error!(
            "@page {  @top-left-corner foo { content: \" \"; border: solid green; } }",
            page,
            LeftCurlyExpected
        );
        // no bueno assert_parse_error!("@page {  @XY foo { content: " "; border: solid green; } }", page, UnknownAtRule);
        assert_parse_error!(
            "@page :left { margin-left: 4cm margin-right: 3cm; }",
            page,
            SemiColonExpected
        );
        assert_parse_error!("@page : { }", page, IdentifierExpected);
        assert_parse_error!("@page :left, { }", page, IdentifierExpected);
    }

    #[test]
    fn at_layer() {
        assert_parse_node!(
            "@layer utilities { .padding-sm { padding: .5rem; } }",
            layer(false)
        );
        assert_parse_node!("@layer utilities;", layer(false));
        assert_parse_node!("@layer theme, layout, utilities;", layer(false));
        assert_parse_node!(
            "@layer utilities { p { margin-block: 1rem; } }",
            layer(false)
        );
        assert_parse_node!("@layer framework { @layer layout { } }", layer(false));
        assert_parse_node!(
            "@layer framework.layout { @keyframes slide-left {} }",
            layer(false)
        );

        assert_parse_node!(
            "@media (min-width: 30em) { @layer layout { } }",
            stylesheet_fall
        );

        assert_parse_error!("@layer theme layout {  }", layer(false), SemiColonExpected);
        assert_parse_error!("@layer theme, layout {  }", layer(false), SemiColonExpected);
        assert_parse_error!(
            "@layer framework .layout {  }",
            layer(false),
            SemiColonExpected
        );
        assert_parse_error!(
            "@layer framework. layout {  }",
            layer(false),
            IdentifierExpected
        );
    }

    #[test]
    fn operator() {
        assert_parse_node!("/", operator);
        assert_parse_node!("*", operator);
        assert_parse_node!("+", operator);
        assert_parse_node!("-", operator);
    }

    #[test]
    fn combinator() {
        assert_parse_node!("+", combinator);
        assert_parse_node!("+  ", combinator);
        assert_parse_node!(">  ", combinator);
        assert_parse_node!(">", combinator);
        assert_parse_node!(">>>", combinator);
        assert_parse_node!("/deep/", combinator);
        assert_parse_node!(":host >>> .data-table { width: 100%; }", stylesheet_fall);
        assert_parse_error!(
            ":host >> .data-table { width: 100%; }",
            stylesheet_fall,
            LeftCurlyExpected
        );
    }

    #[test]
    fn unary_operator() {
        assert_parse_node!("-", unary_operator);
        assert_parse_node!("+", unary_operator);
    }

    #[test]
    fn property() {
        assert_parse_node!("asdsa", property);
        assert_parse_node!("asdsa334", property);

        assert_parse_node!("--color", property);
        assert_parse_node!("--primary-font", property);
        assert_parse_node!("-color", property);
        assert_parse_node!("somevar", property);
        assert_parse_node!("some--let", property);
        assert_parse_node!("somevar--", property);
    }

    #[test]
    fn ruleset() {
        assert_parse_node!("name{ }", rule_set(false));
        assert_parse_node!("	name\n{ some : \"asdas\" }", rule_set(false));
        assert_parse_node!("		name{ some : \"asdas\" !important }", rule_set(false));
        assert_parse_node!(
            "name{ \n some : \"asdas\" !important; some : \"asdas\" }",
            rule_set(false)
        );
        assert_parse_node!("* {}", rule_set(false));
        assert_parse_node!(".far{}", rule_set(false));
        assert_parse_node!("boo {}", rule_set(false));
        assert_parse_node!(".far #boo {}", rule_set(false));
        assert_parse_node!("boo { prop: value }", rule_set(false));
        assert_parse_node!("boo { prop: value; }", rule_set(false));
        assert_parse_node!("boo { prop: value; prop: value }", rule_set(false));
        assert_parse_node!("boo { prop: value; prop: value; }", rule_set(false));
        assert_parse_node!("boo {--minimal: }", rule_set(false));
        assert_parse_node!("boo {--minimal: ;}", rule_set(false));
        assert_parse_node!("boo {--normal-text: red yellow green}", rule_set(false));
        assert_parse_node!("boo {--normal-text: red yellow green;}", rule_set(false));
        assert_parse_node!("boo {--important: red !important;}", rule_set(false));
        assert_parse_node!("boo {--nested: {color: green;}}", rule_set(false));
        assert_parse_node!("boo {--parens: this()is()ok()}", rule_set(false));
        assert_parse_node!("boo {--squares: this[]is[]ok[]too[]}", rule_set(false));
        assert_parse_node!("boo {--combined: ([{{[]()()}[]{}}])()}", rule_set(false));
        assert_parse_node!(
            "boo {--weird-inside-delims: {color: green;;;;;;!important;;}}",
            rule_set(false)
        );
        assert_parse_node!("boo {--validValue: , 0 0}", rule_set(false));
        assert_parse_node!("boo {--validValue: , 0 0;}", rule_set(false));
        assert_parse_error!("boo, { }", rule_set(false), SelectorExpected);
    }

    #[test]
    fn ruleset_panic() {
        // no bueno assert_parse_node!("boo { : value }", rule_set(false));
        assert_parse_error!("boo { prop: ; }", rule_set(false), PropertyValueExpected);
        assert_parse_error!("boo { prop }", rule_set(false), ColonExpected);
        assert_parse_error!(
            "boo { prop: ; far: 12em; }",
            rule_set(false),
            PropertyValueExpected
        );
        //no bueno assert_parse_node!("boo { prop: ; 1ar: 12em; }", rule_set(false));

        assert_parse_error!(
            "boo { --too-minimal:}",
            rule_set(false),
            PropertyValueExpected
        );
        assert_parse_error!(
            "boo { --unterminated: ",
            rule_set(false),
            RightCurlyExpected
        );
        assert_parse_error!(
            "boo { --double-important: red !important !important;}",
            rule_set(false),
            SemiColonExpected
        );
        assert_parse_error!(
            "boo {--unbalanced-curlys: {{color: green;}}",
            rule_set(false),
            RightCurlyExpected
        );
        assert_parse_error!(
            "boo {--unbalanced-parens: not(()cool;}",
            rule_set(false),
            LeftCurlyExpected
        );
        assert_parse_error!(
            "boo {--unbalanced-parens: not)()(cool;}",
            rule_set(false),
            LeftParenthesisExpected
        );
        assert_parse_error!(
            "boo {--unbalanced-brackets: not[[]valid;}",
            rule_set(false),
            LeftCurlyExpected
        );
        assert_parse_error!(
            "boo {--unbalanced-brackets: not][][valid;}",
            rule_set(false),
            LeftSquareBracketExpected
        );
    }

    #[test]
    fn nested_ruleset() {
        assert_parse_node!(
            ".foo { color: red; input { color: blue; } }",
            rule_set(false)
        );
        assert_parse_node!(
            ".foo { color: red; :focus { color: blue; } }",
            rule_set(false)
        );
        assert_parse_node!(
            ".foo { color: red; .bar { color: blue; } }",
            rule_set(false)
        );
        assert_parse_node!(
            ".foo { color: red; &:hover { color: blue; } }",
            rule_set(false)
        );
        assert_parse_node!(
            ".foo { color: red; + .bar { color: blue; } }",
            rule_set(false)
        );
        assert_parse_node!(
            ".foo { color: red; foo:hover { color: blue }; }",
            rule_set(false)
        );
        assert_parse_node!(
            ".foo { color: red; @media screen { color: blue }; }",
            rule_set(false)
        );

        // Top level curly braces are allowed in declaration values if they are for a custom property.
        assert_parse_node!(".foo { --foo: {}; }", rule_set(false));
        // Top level curly braces are not allowed in declaration values.
        assert_parse_error!(".foo { foo: {}; }", rule_set(false), PropertyValueExpected);
    }

    #[test]
    fn nested_ruleset_2() {
        assert_parse_node!(".foo { .parent & { color: blue; } }", rule_set(false));
        assert_parse_node!(
            ".foo { color: red; & > .bar, > .baz { color: blue; } }",
            rule_set(false)
        );
        assert_parse_node!(
            ".foo { & .bar & .baz & .qux { color: blue; } }",
            rule_set(false)
        );
        assert_parse_node!(
            ".foo { color: red; :not(&) { color: blue; }; + .bar + & { color: green; } }",
            rule_set(false)
        );
        assert_parse_node!(
            ".foo { color: red; & { color: blue; } && { color: green; } }",
            rule_set(false)
        );
        assert_parse_node!(
            ".foo { & :is(.bar, &.baz) { color: red; } }",
            rule_set(false)
        );
        assert_parse_node!("figure { > figcaption { background: hsl(0 0% 0% / 50%); > p {  font-size: .9rem; } } }", rule_set(false));
        assert_parse_node!(
            "@layer base { html { & body { min-block-size: 100%; } } }",
            stylesheet_fall
        );
    }

    #[test]
    fn selector() {
        assert_parse_node!("asdsa", selector(false));
        assert_parse_node!("asdsa + asdas", selector(false));
        assert_parse_node!("asdsa + asdas + name", selector(false));
        assert_parse_node!("asdsa + asdas + name", selector(false));
        assert_parse_node!("name #id#anotherid", selector(false));
        assert_parse_node!("name.far .boo", selector(false));
        assert_parse_node!("name .name .zweitername", selector(false));
        assert_parse_node!("*", selector(false));
        assert_parse_node!("#id", selector(false));
        assert_parse_node!("far.boo", selector(false));
        assert_parse_node!("::slotted(div)::after", selector(false)); // 35076
    }

    #[test]
    fn simple_selector() {
        assert_parse_node!("name", simple_selector);
        assert_parse_node!("#id#anotherid", simple_selector);
        assert_parse_node!("name.far", simple_selector);
        assert_parse_node!("name.erstername.zweitername", simple_selector);
    }

    #[test]
    fn element_name() {
        assert_parse_node!("name", element_name);
        assert_parse_node!("*", element_name);
        assert_parse_node!("foo|h1", element_name);
        assert_parse_node!("foo|*", element_name);
        assert_parse_node!("|h1", element_name);
        assert_parse_node!("*|h1", element_name);
    }

    #[test]
    fn attrib() {
        assert_parse_node!("[name]", attribute);
        assert_parse_node!("[name = name2]", attribute);
        assert_parse_node!("[name ~= name3]", attribute);
        assert_parse_node!("[name~=name3]", attribute);
        assert_parse_node!("[name |= name3]", attribute);
        assert_parse_node!("[name |= \"this is a striiiing\"]", attribute);
        assert_parse_node!("[href*=\"insensitive\" i]", attribute);
        assert_parse_node!("[href*=\"sensitive\" S]", attribute);

        // Single namespace
        assert_parse_node!("[namespace|name]", attribute);
        assert_parse_node!("[name-space|name = name2]", attribute);
        assert_parse_node!("[name_space|name ~= name3]", attribute);
        assert_parse_node!("[name0spae|name~=name3]", attribute);
        assert_parse_node!("[NameSpace|name |= \"this is a striiiing\"]", attribute);
        assert_parse_node!("[name\\*space|name |= name3]", attribute);
        assert_parse_node!("[*|name]", attribute);
    }

    #[test]
    fn pseudo() {
        assert_parse_node!(":some", pseudo);
        assert_parse_node!(":some(thing)", pseudo);
        assert_parse_node!(":nth-child(12)", pseudo);
        assert_parse_node!(":nth-child(1n)", pseudo);
        assert_parse_node!(":nth-child(-n+3)", pseudo);
        assert_parse_node!(":nth-child(2n+1)", pseudo);
        assert_parse_node!(":nth-child(2n+1 of .foo)", pseudo);
        assert_parse_node!(
            ":nth-child(2n+1 of .foo > bar, :not(*) ~ [other=\"value\"])",
            pseudo
        );
        assert_parse_node!(":lang(it)", pseudo);
        assert_parse_node!(":not(.class)", pseudo);
        assert_parse_node!(":not(:disabled)", pseudo);
        assert_parse_node!(":not(#foo)", pseudo);
        assert_parse_node!("::slotted(*)", pseudo); // #35076
        assert_parse_node!("::slotted(div:hover)", pseudo); // #35076
        assert_parse_node!(":global(.output ::selection)", pseudo); // #49010
        assert_parse_node!(":matches(:hover, :focus)", pseudo); // #49010
        assert_parse_node!(":host([foo=bar][bar=foo])", pseudo); // #49589
        assert_parse_node!(":has(> .test)", pseudo); // #250
        assert_parse_node!(":has(~ .test)", pseudo); // #250
        assert_parse_node!(":has(+ .test)", pseudo); // #250
        assert_parse_node!(":has(~ div .test)", pseudo); // #250
        assert_parse_error!("::", pseudo, IdentifierExpected);
        assert_parse_error!(":: foo", pseudo, IdentifierExpected);
        assert_parse_error!(":nth-child(1n of)", pseudo, SelectorExpected);
    }

    #[test]
    fn declaration() {
        assert_parse_node!("name : \"this is a string\" !important", declaration(None));
        assert_parse_node!("name : \"this is a string\"", declaration(None));
        assert_parse_node!("property:12", declaration(None));
        assert_parse_node!("-vendor-property: 12", declaration(None));
        assert_parse_node!("font-size: 12px", declaration(None));
        assert_parse_node!("color : #888 /4", declaration(None));
        assert_parse_node!(
            "filter : progid:DXImageTransform.Microsoft.Shadow(color=#000000,direction=45)",
            declaration(None)
        );
        assert_parse_node!("filter : progid: DXImageTransform.\nMicrosoft.\nDropShadow(\noffx=2, offy=1, color=#000000)", declaration(None));
        assert_parse_node!("font-size: 12px", declaration(None));
        assert_parse_node!("*background: #f00 /* IE 7 and below */", declaration(None));
        assert_parse_node!("_background: #f60 /* IE 6 and below */", declaration(None));
        assert_parse_node!("background-image: linear-gradient(to right, silver, white 50px, white calc(100% - 50px), silver)", declaration(None));
        assert_parse_node!(
            "grid-template-columns: [first nav-start] 150px [main-start] 1fr [last]",
            declaration(None)
        );
        assert_parse_node!(
            "grid-template-columns: repeat(4, 10px [col-start] 250px [col-end]) 10px",
            declaration(None)
        );
        assert_parse_node!("grid-template-columns: [a] auto [b] minmax(min-content, 1fr) [b c d] repeat(2, [e] 40px)", declaration(None));
        assert_parse_node!("grid-template: [foo] 10px / [bar] 10px", declaration(None));
        assert_parse_node!(
            "grid-template: 'left1 footer footer' 1fr [end] / [ini] 1fr [info-start] 2fr 1fr [end]",
            declaration(None)
        );
        assert_parse_node!("content: \"(\"counter(foo) \")\"", declaration(None));
        assert_parse_node!("content: 'Hello\\0A''world'", declaration(None));
    }

    #[test]
    fn term() {
        assert_parse_node!("\"asdasd\"", term);
        assert_parse_node!("name", term);
        assert_parse_node!("#FFFFFF", term);
        assert_parse_node!("url(\"this is a url\")", term);
        assert_parse_node!("+324", term);
        assert_parse_node!("-45", term);
        assert_parse_node!("+45", term);
        assert_parse_node!("-45%", term);
        assert_parse_node!("-45mm", term);
        assert_parse_node!("-45em", term);
        assert_parse_node!("\"asdsa\"", term);
        assert_parse_node!("faa", term);
        assert_parse_node!("url(\"this is a striiiiing\")", term);
        assert_parse_node!("#FFFFFF", term);
        assert_parse_node!("name(asd)", term);
        assert_parse_node!("calc(50% + 20px)", term);
        assert_parse_node!("calc(50% + (100%/3 - 2*1em - 2*1px))", term);
        assert_no_node(
            "%('repetitions: %S file: %S', 1 + 2, \"directory/file.less\")",
            |parser: &mut Parser| parser.parse_term(),
        ); // less syntax
        assert_no_node(
            "~\"ms:alwaysHasItsOwnSyntax.For.Stuff()\"",
            |parser: &mut Parser| parser.parse_term(),
        ); // less syntax
        assert_parse_node!("U+002?-0199", term);
        assert_no_node("U+002?-01??", |parser: &mut Parser| parser.parse_term());
        assert_no_node("U+00?0;", |parser: &mut Parser| parser.parse_term());
        assert_no_node("U+0XFF;", |parser: &mut Parser| parser.parse_term());
    }

    #[test]
    fn function() {
        assert_parse_node!("name( \"bla\" )", function);
        assert_parse_node!("name( name )", function);
        assert_parse_node!("name( -500mm )", function);
        assert_parse_node!("\u{060f}rf()", function);
        assert_parse_node!("über()", function);

        assert_no_node("über ()", |parser: &mut Parser| parser.parse_function());
        assert_no_node("%()", |parser: &mut Parser| parser.parse_function());
        assert_no_node("% ()", |parser: &mut Parser| parser.parse_function());

        assert_parse_node!("let(--color)", function);
        assert_parse_node!("let(--color, somevalue)", function);
        assert_parse_node!("let(--variable1, --variable2)", function);
        assert_parse_node!("let(--variable1, let(--variable2))", function);
        assert_parse_node!("fun(value1, value2)", function);
        assert_parse_node!("fun(value1,)", function);
    }

    #[test]
    fn test_token_prio() {
        assert_parse_node!("!important", prio);
        assert_parse_node!("!/*demo*/important", prio);
        assert_parse_node!("! /*demo*/ important", prio);
        assert_parse_node!("! /*dem o*/  important", prio);
    }

    #[test]
    fn hexcolor() {
        assert_parse_node!("#FFF", hex_color);
        assert_parse_node!("#FFFF", hex_color);
        assert_parse_node!("#FFFFFF", hex_color);
        assert_parse_node!("#FFFFFFFF", hex_color);
    }

    #[test]
    fn test_class() {
        assert_parse_node!(".faa", class);
        assert_parse_node!("faa", element_name);
        assert_parse_node!("*", element_name);
        assert_parse_node!(".faa42", class);
    }

    #[test]
    fn prio() {
        assert_parse_node!("!important", prio);
    }

    #[test]
    fn expr() {
        assert_parse_node!("45,5px", expr(false));
        assert_parse_node!(" 45 , 5px ", expr(false));
        assert_parse_node!("5/6", expr(false));
        assert_parse_node!("36mm, -webkit-calc(100%-10px)", expr(false));
    }

    #[test]
    fn url() {
        assert_parse_node!("url(//yourdomain/yourpath.png)", uri_literal);
        assert_parse_node!("url('http://msft.com')", uri_literal);
        assert_parse_node!("url(\"http://msft.com\")", uri_literal);
        assert_parse_node!("url( \"http://msft.com\")", uri_literal);
        assert_parse_node!("url(\t\"http://msft.com\")", uri_literal);
        assert_parse_node!("url(\n\"http://msft.com\")", uri_literal);
        assert_parse_node!("url(\"http://msft.com\"\n)", uri_literal);
        assert_parse_node!("url(\"\")", uri_literal);
        assert_parse_node!("uRL(\"\")", uri_literal);
        assert_parse_node!("URL(\"\")", uri_literal);
        assert_parse_node!("url(http://msft.com)", uri_literal);
        assert_parse_node!("url()", uri_literal);
        assert_parse_node!("url('http://msft.com\n)", uri_literal);
        assert_parse_error!(
            "url(\"http://msft.com\"",
            uri_literal,
            RightParenthesisExpected
        );
        assert_parse_error!(
            "url(http://msft.com')",
            uri_literal,
            RightParenthesisExpected
        );
    }
}
