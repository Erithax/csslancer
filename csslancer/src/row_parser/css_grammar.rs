// TODO: don't emit an error node, but emit parsed node with nested error

#![allow(clippy::unit_arg)]
#![allow(clippy::collapsible_if)]

use ungrammar::Token;

use super::parse_error::ParseError;
use super::parser::{Parser, Marker};
use super::syntax_kind_gen::SyntaxKind;
use super::token_set::TokenSet;
use super::Parse;
use crate::T;

pub enum ReferenceType {
    Mixin,
    Rule,
    Variable,
    Function,
    Keyframe,
    Unknown,
    Module,
    Forward,
    ForwardVisibility,
    Property,
}

impl Parser<'_> {

    // pub fn ttry<T, F>(&mut self, func: F) -> Option<T>
    // where
    //     F: Fn(&mut Self) -> Option<T>,
    // {
    //     let saved = self.start();
    //     match func(self) {
    //         Some(n) => {
    //             saved.complete(self, SyntaxKind::UNDEFINED);
    //             Some(n)
    //         },
    //         None => {
    //             saved.rollback(self);
    //             None
    //         }
    //     }
    // }

    pub fn varnish(&mut self, m: Marker, s: SyntaxKind) {
        m.complete(self, s);
    }

    // pub fn finito(
    //     &mut self,
    //     m: Marker,
    //     error: ParseError,
    // ) {
    //     self.err_and_bump(error.issue().desc);
    //     m.complete(self, SyntaxKind::ERROR);
    // }

    pub fn err_pe(&mut self, error: ParseError) {
        self.error(error.issue().desc);
    }

    pub fn err_and_bump_pe(
        &mut self,
        error: ParseError,
    ) {
        self.err_and_bump(error.issue().desc);
    }

    // pub fn fintio_recover(
    //     &mut self,
    //     m: Marker,
    //     error: ParseError,
    //     resync_tokens: Option<&[SyntaxKind]>,
    //     resync_stop_tokens: Option<&[SyntaxKind]>,
    // ) {
    //     self.error(error.issue().desc);
    //     self.resync(resync_tokens.unwrap_or(&[]), resync_stop_tokens.unwrap_or(&[]));
    //     m.complete(self, SyntaxKind::ERROR); // No rollback here! creates infinite loops on error (because nothing is consumed)
    // }

    pub fn err_resync_pe(
        &mut self,
        error: ParseError,
        resync_tokens: Option<TokenSet>,
        resync_stop_tokens: Option<TokenSet>,
    ) -> bool {
        self.err_resync(error.issue().desc, resync_tokens.unwrap_or(TokenSet::EMPTY), resync_stop_tokens.unwrap_or(TokenSet::EMPTY))
        //self.error(error.issue().desc);
        //self.resync(resync_tokens.unwrap_or(&[]), resync_stop_tokens.unwrap_or(&[]));
    }

    // #[deprecated]
    // pub fn fintio_recover_nested_error(
    //     &mut self,
    //     error: ParseError,
    //     resync_tokens: Option<&[SyntaxKind]>,
    //     resync_stop_tokens: Option<&[SyntaxKind]>,
    // ) -> bool {
    //     let m = self.start();
    //     self.error(error.issue().desc);
    //     let resynced = self.resync(resync_tokens.unwrap_or(&[]), resync_stop_tokens.unwrap_or(&[]));
    //     m.complete(self, SyntaxKind::ERROR);
    //     resynced
    // }

    // Returns if resync was succesful, or not (EOF reached)
    // pub fn resync(
    //     &mut self,
    //     resync_tokens: &[SyntaxKind],
    //     resync_stop_tokens: &[SyntaxKind],
    // ) -> bool {
    //     loop {
    //         if self.current() == SyntaxKind::EOF {
    //             return false;
    //         }
    //         if resync_tokens.contains(&self.current()) {
    //             self.bump_any();
    //             return true;
    //         } else if resync_stop_tokens.contains(&self.current()) {
    //             return true;
    //         }
    //         self.bump_any();
    //     }
    // }
    
}

impl Parser<'_> {
    pub fn parse_source_file(&mut self) {
        let m = self.start();

        self.parse_charset_opt();

        let mut in_recovery = false;
        let mut has_match;
        loop {
            loop {
                has_match = false;
                if let Some(kind) = self.parse_stylesheet_statement_opt(false) {
                    has_match = true;
                    in_recovery = false;
                    if !self.at(SyntaxKind::EOF)
                        && Self::needs_semicolon_after(kind)
                        && !self.eat(T![;])
                    {
                        self.error(ParseError::SemiColonExpected.issue().desc);
                    }
                }
                while self.eat(T![;])
                    || self.eat(T![cdo])
                    || self.eat(T![cdc])
                {
                    // accept empty statements
                    has_match = true;
                    in_recovery = false;
                }

                if !has_match {
                    break;
                }
            }
            if self.at(SyntaxKind::EOF) {
                break;
            }

            if !in_recovery {
                if self.current().is_at_keyword() {
                    self.error(ParseError::UnknownAtRule.issue().desc);
                } else {
                    self.error(ParseError::RuleOrSelectorExpected.issue().desc);
                }
                in_recovery = true;
            }
            self.bump_any();

            if self.at(SyntaxKind::EOF) {
                break;
            }
        }
        
        m.complete(self, SyntaxKind::SOURCE_FILE);
    }

    pub fn parse_charset_opt(&mut self) -> Option<()> {
        if !self.eat(SyntaxKind::CHARSET) {
            return None
        }
        Some(())
    }

    pub fn parse_stylesheet_statement_opt(&mut self, is_nested: bool) -> Option<SyntaxKind> {
        if self.current().is_at_keyword() {
            return self.parse_stylesheet_at_statement_opt(is_nested)
        }
        self.parse_rule_set_opt(is_nested).map(|_| SyntaxKind::RULE_SET)
    }

    pub fn parse_stylesheet_at_statement_opt(&mut self, is_nested: bool) -> Option<SyntaxKind> {
        self
            .parse_import_opt().map(|_| SyntaxKind::IMPORT)
            .or_else(|| self.parse_media_opt(is_nested).map(|_| SyntaxKind::MEDIA))
            .or_else(|| self.parse_page().map(|_| SyntaxKind::PAGE))
            .or_else(|| self.parse_font_face_opt().map(|_| SyntaxKind::FONT_FACE))
            .or_else(|| self.parse_keyframe_opt().map(|_| SyntaxKind::KEYFRAME))
            .or_else(|| self.parse_supports_opt(is_nested).map(|_| SyntaxKind::SUPPORTS))
            .or_else(|| self.parse_layer_opt(is_nested).map(|_| SyntaxKind::LAYER))
            .or_else(|| self.parse_property_at_rule_opt().map(|_| SyntaxKind::PROPERTY_AT_RULE))
            .or_else(|| self.parse_viewport_opt().map(|_| SyntaxKind::VIEW_PORT))
            .or_else(|| self.parse_namespace_opt().map(|_| SyntaxKind::NAMESPACE))
            .or_else(|| self.parse_document_opt().map(|_| SyntaxKind::DOCUMENT))
            .or_else(|| self.parse_container().map(|_| SyntaxKind::CONTAINER))
            .or_else(|| self.parse_unknown_at_rule().map(|_| SyntaxKind::UNKNOWN_AT_RULE))
    }


    pub fn try_parse_rule_set_opt(&mut self, is_nested: bool) -> Option<()> {
        let m = self.start();
        if self.parse_selector_opt(is_nested).is_none() {
            m.rollback(self);
            return None
        }
        while self.eat(T![,]) && self.parse_selector_opt(is_nested).is_some() {
            // consume comma seperated selectors
        }
        if !self.eat(SyntaxKind::L_CURLY) {
            m.rollback(self);
            return None
        } 
        m.rollback(self); // Rollback before real parse_rule_set
        self.parse_rule_set_opt(is_nested)
    }

    pub fn parse_rule_set_opt(&mut self, is_nested: bool) -> Option<()> {
        let m = self.start();
        if self.parse_selector_opt(is_nested).is_none() {
            m.rollback(self);
            return None
        }

        while self.eat(T![,]) {
            if self.parse_selector_opt(is_nested).is_none() {
                self.err_resync_pe(ParseError::SelectorExpected, None, None);
                // self.finito(m, ParseError::SelectorExpected);
                // return Some(())
            }
        }

        self.parse_body(|s: &mut Self| s.parse_rule_set_declaration_opt());
        Some(self.varnish(m, SyntaxKind::RULE_SET))
    }

    pub fn parse_rule_set_declaration_at_statement_opt(&mut self) -> Option<SyntaxKind> {
        self
            .parse_media_opt(true).map(|_| SyntaxKind::MEDIA)
            .or_else(|| self.parse_supports_opt(true).map(|_| SyntaxKind::SUPPORTS))
            .or_else(|| self.parse_layer_opt(true).map(|_| SyntaxKind::LAYER))
            .or_else(|| self.parse_unknown_at_rule().map(|_| SyntaxKind::UNKNOWN_AT_RULE))
    }

    pub fn parse_rule_set_declaration_opt(&mut self) -> Option<SyntaxKind> {
        // https://www.w3.org/TR/css-syntax-3/#consume-a-list-of-declarations
        if self.current().is_at_keyword() {
            return self.parse_rule_set_declaration_at_statement_opt();
        }
        if !self.at(T![identifier]) {
            return self.parse_rule_set_opt(true).map(|_| SyntaxKind::RULE_SET)
        }
        self
            .try_parse_rule_set_opt(true).map(|_| SyntaxKind::RULE_SET)
            .or_else(|| self.parse_declaration_opt(None).map(|_| SyntaxKind::DECLARATION))
    }

    pub fn needs_semicolon_after(sk: SyntaxKind) -> bool {
        // SHOULD DO: match exhaustively instead of with default (panic on not explicitely handled)
        match sk {
            //BODY DECLARATION
            SyntaxKind::KEYFRAME
            | SyntaxKind::UNKNOWN_AT_RULE
            | SyntaxKind::KEYFRAME_SELECTOR
            | SyntaxKind::VIEW_PORT
            | SyntaxKind::MEDIA
            | SyntaxKind::PAGE
            | SyntaxKind::PAGE_BOX_MARGIN_BOX
            | SyntaxKind::RULE_SET
            | SyntaxKind::SCSS_IF_STATEMENT
            | SyntaxKind::SCSS_FOR_STATEMENT
            | SyntaxKind::SCSS_EACH_STATEMENT
            | SyntaxKind::SCSS_WHILE_STATEMENT
            | SyntaxKind::XCSS_MIXIN_DECLARATION
            | SyntaxKind::SCSS_FUNCTION_DECLARATION
            | SyntaxKind::SCSS_MIXIN_CONTENT_DECLARATION 
            | SyntaxKind::PROPERTY_AT_RULE
            | SyntaxKind::CONTAINER
            | SyntaxKind::FONT_FACE
            | SyntaxKind::LAYER
            | SyntaxKind::SUPPORTS
            // --
            | SyntaxKind::NAMESPACE => false,
            SyntaxKind::MEDIA_QUERY
            | SyntaxKind::IMPORT
            | SyntaxKind::XCSS_EXTENDS_REFERENCE
            | SyntaxKind::SCSS_MIXIN_CONTENT_REFERENCE
            | SyntaxKind::SCSS_RETURN_STATEMENT
            | SyntaxKind::SCSS_DEBUG
            //| SyntaxKind::AT_APPLY_RULE 
            => true,
            SyntaxKind::DECLARATION_XCSS_VARIABLE => todo!("need to call self.needs_semicolon"),
            SyntaxKind::XCSS_MIXIN_REFERENCE => todo!("need to call self.content.is_none()"),
            // declaration common
            SyntaxKind::DECLARATION => panic!("no call Parser::needs_semicolon on DECLARATION! call on variants instead"),
            SyntaxKind::DECLARATION_CUSTOM_PROPERTY => true,
            SyntaxKind::DECLARATION_BASIC => false, // todo!("xcss: need to call self.nested_properties.is_none()"),
            // --
            _ => panic!("unhandled Parser::needs_semicolon_after(sk: {:?})", sk),
        }
    }

    /// `parse_declaration_func` must return Option<NodeId> of node of type `_AbstractDeclaration`
    pub fn parse_declarations<F>(&mut self, mut parse_declaration_func: F) -> Option<()>
    where
        F: FnMut(&mut Self) -> Option<SyntaxKind>,
    {
        let m = self.start();
        if !self.eat(SyntaxKind::L_CURLY) {
            m.rollback(self);
            return None
        }

        while let Some(mut kind) = parse_declaration_func(self) {
            if self.at(SyntaxKind::R_CURLY) {
                break;
            }

            if kind == SyntaxKind::DECLARATION {
                kind = SyntaxKind::DECLARATION_BASIC;
                // FIXME: should check inner declaration type
                
            }

            if Self::needs_semicolon_after(kind) && !self.eat(T![;]) {
				self.err_resync_pe(ParseError::SemiColonExpected, Some(TokenSet::new(&[T![;], SyntaxKind::R_CURLY])), None);
                m.complete(self, SyntaxKind::DECLARATIONS);
                return Some(())
			}

            while self.eat(T![;]) {
                // accept empty statements
            }
        }
        if !self.eat(SyntaxKind::R_CURLY) {
            //self.err_resync_pe(ParseError::RightCurlyExpected, Some(TokenSet::new(&[SyntaxKind::R_CURLY, T![;]])), None);
            // FIXME: testing if this resync causes incremental reparsing errors, above line was the original, below line didn't fix the reparsing
            self.err_pe(ParseError::RightCurlyExpected);
        }
        m.complete(self, SyntaxKind::DECLARATIONS);
        Some(())
    }

    // node.node_type.is_body_declaration() == true
    /// `parse_declaration_func` must return Option<NodeId> which has node type `_AbstractDeclaration`
    pub fn parse_body<F>(&mut self, parse_declaration_func: F)
    where
        F: FnMut(&mut Self) -> Option<SyntaxKind>
    {
        //let m = self.start();

        if self.parse_declarations(parse_declaration_func).is_none() {
            self.err_resync_pe(ParseError::LeftCurlyExpected, Some(TokenSet::new(&[SyntaxKind::R_CURLY, T![;]])), None);
        }
        
        //m.complete(self, SyntaxKind::UNDEFINED);
        //m.abandon(self); // attach child DECLARATIONS to parent as is. Incremental reparsing relies on this.
    }

    pub fn parse_selector_opt(&mut self, is_nested: bool) -> Option<()> {
        let m = self.start();

        let mut has_content = false;
        if is_nested {
            // nested selectors can start with a combinator
            has_content = self.parse_combinator_opt().is_some();
        }
        while self.parse_simple_selector().is_some() {
            has_content = true;
            self.parse_combinator_opt();
        }
        if !has_content {
            m.rollback(self);
            return None
        }
        m.complete(self, SyntaxKind::SELECTOR);
        Some(())
    }

    pub fn parse_declaration_opt(&mut self, stop_tokens: Option<&[SyntaxKind]>) -> Option<()> {
        let m = self.start();
        if self.try_parse_custom_property_declaration_opt(stop_tokens).is_some() {
            return Some(self.varnish(m, SyntaxKind::DECLARATION))
        }

        let d = self.start();

        if self.parse_property_opt().is_none() {
            d.rollback(self);
            m.rollback(self);
            return None
        }

        if !self.eat(T![:]) {
            //d.rollback(self);
            self.err_resync_pe(ParseError::ColonExpected, Some(TokenSet::new(&[T![:]])), Some(TokenSet::new(&[T![;]])));
        }

        if self.parse_expr_opt(false).is_none() {
            //d.rollback(self);
            //return Some(self.finito(m, ParseError::PropertyValueExpected))
            self.err_and_bump_pe(ParseError::PropertyValueExpected);
        }

        self.parse_prio_opt();

        // if !self.eat(T![;]) {
        //     todo!("to err or not to err");
        //     return Some(self.finito(m, ParseError::SemiColonExpected))
        // }

        self.varnish(d, SyntaxKind::DECLARATION_BASIC);

        Some(self.varnish(m, SyntaxKind::DECLARATION))
    }

    pub fn try_parse_custom_property_declaration_opt(
        &mut self,
        stop_tokens: Option<&[SyntaxKind]>,
    ) -> Option<()> {
        assert!(stop_tokens.is_none() || stop_tokens.is_some_and(|s| s.iter().all(|t| t.is_punct())));
        if !self.at_contextual_token(T![cxid_valid_custom_prop]) {
            return None
        }
        // if !self.peek_regex(T![identifier], Regex::new("^--").unwrap()) {
        //     return None
        // }

        let m = self.start();

        if self.parse_property_opt().is_none() {
            m.rollback(self);
            return None
        }

        if !self.eat(T![:]) {
            // todo? try resync to ':' and continue, else try resyncstop to ';' and return even if it fails
            self.err_resync_pe(ParseError::ColonExpected, Some(TokenSet::new(&[T![:]])), Some(TokenSet::new(&[T![;]])));
            //return Some(self.fintio_recover(m, ParseError::ColonExpected, Some(&[T![:]]), None))
        }
        let has_whitespace_after_colon = self.has_whitespace();

        // try to parse it as nested declaration
        if self.at(SyntaxKind::L_CURLY) {
            let prop_set = self.start();
            if 
                self.parse_declarations(|s: &mut Self| s.parse_rule_set_declaration_opt()).is_some() &&
                !self.did_err_since_last_unfinished() 
            {
                self.parse_prio_opt();
                if self.at(T![;]) {
                    prop_set.complete(self, SyntaxKind::CUSTOM_PROPERTY_SET);
                    //assert!(self.eat(T![;])); // not part of the declaration, but useful information for code assist
                    
                    m.complete(self, SyntaxKind::DECLARATION_CUSTOM_PROPERTY);
                    return Some(())
                }
            }
            prop_set.rollback(self);
        }

        // try to parse as expression
        let expr = self.start();
        if self.parse_expr_opt(false).is_some() {
            if !self.did_err_since_last_unfinished() {
                self.parse_prio_opt();
                let mut toks = vec![T![;], SyntaxKind::EOF];
                toks.append(&mut stop_tokens.unwrap_or(&[]).to_vec());
                if toks.into_iter().any(|t| self.at(t)) {
                    //assert!(self.eat(T![;]));
                    expr.abandon(self);
                    m.complete(self, SyntaxKind::DECLARATION_CUSTOM_PROPERTY);
                    return Some(())
                }
            }
        }
        expr.rollback(self);

        let prev_pos = self.pos();
        
        self.parse_custom_property_value(stop_tokens.unwrap_or(&[SyntaxKind::R_CURLY]));

        if !has_whitespace_after_colon && prev_pos == self.pos() {
            self.err_and_bump_pe(ParseError::PropertyValueExpected);
        }

        m.complete(self, SyntaxKind::DECLARATION_CUSTOM_PROPERTY);
        Some(())
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
    pub fn parse_custom_property_value(&mut self, stop_tokens: &[SyntaxKind]) {
        let m = self.start();
        let mut curly_dep: i32 = 0;
        let mut paren_dep: i32 = 0;
        let mut brack_dep: i32 = 0;
        macro_rules! on_stop_token {
            () => {
                stop_tokens.iter().any(|&st| self.at(st))
            };
        }
        macro_rules! is_top_lvl {
            () => {
                curly_dep == 0 && paren_dep == 0 && brack_dep == 0
            };
        }
        loop {
            match self.current() {
                T![;] | T![!] => {
                    if is_top_lvl!() {
                        // exclamation or semicolon ends things if we are not inside delims
                        break;
                    }
                }
                SyntaxKind::L_CURLY => curly_dep += 1,
                SyntaxKind::R_CURLY => {
                    curly_dep -= 1;
                    if curly_dep < 0 {
                        // The property value has been terminated without a semicolon,
                        // and this is the last declaration in the ruleset
                        if on_stop_token!() && paren_dep == 0 && brack_dep == 0 {
                            break;
                        }
                        self.err_pe(ParseError::LeftCurlyExpected);
                        break
                    }
                }
                SyntaxKind::L_PAREN | T![function] => paren_dep += 1,
                SyntaxKind::R_PAREN => {
                    paren_dep -= 1;
                    if paren_dep < 0 {
                        if on_stop_token!() && brack_dep == 0 && curly_dep == 0 {
                            break;
                        }
                        self.err_pe(ParseError::LeftParenthesisExpected);
                        break;
                    }
                }
                SyntaxKind::L_BRACK => brack_dep += 1,
                SyntaxKind::R_BRACK => {
                    brack_dep -= 1;
                    if brack_dep < 0 {
                        self.err_pe(ParseError::LeftSquareBracketExpected);
                        break;
                    }
                }
                T![bad_string] => break,
                SyntaxKind::EOF => {
                    // we should not have reached the end of input,
                    // something is unterminated
                    let error = if brack_dep > 0 {
                        ParseError::RightSquareBracketExpected
                    } else if paren_dep > 0 {
                        ParseError::RightParenthesisExpected
                    } else {
                        ParseError::RightCurlyExpected
                    };
                    self.err_pe(error);
                    break;
                }
                _ => {
                    // Consume all the rest
                }
            }
            self.bump_any();
        }
        self.varnish(m, SyntaxKind::UNDEFINED)
    }

    pub fn try_parse_declaration_opt(&mut self, stop_tokens: Option<&[SyntaxKind]>) -> Option<()> {
        let m = self.start();
        if self.parse_property_opt().is_some() && self.at(T![:]) {
            // looks like a declaration, rollback and go ahead with real parse
            m.rollback(self);
            return self.parse_declaration_opt(stop_tokens)
        }
        m.rollback(self);
        None
    }

    pub fn parse_property_opt(&mut self) -> Option<()> {
        let m = self.start();
        if self.eat(T![*]) || self.eat(T![_]) {
            // support for IE 5.x. 6, and 7 hack: see http://en.wikipedia.org/wiki/CSS_filter#Star_hack
            if self.has_whitespace() {
                m.rollback(self);
                return None
            }
        }
        if self.parse_property_identifier_opt().is_none() {
            m.rollback(self);
            return None
        }
        m.complete(self, SyntaxKind::PROPERTY);
        Some(())
    }

    #[inline]
    pub fn parse_property_identifier_opt(&mut self) -> Option<()> {
        self.parse_ident_opt(None)
    }

    pub fn parse_import_opt(&mut self) -> Option<()> {
        // @import [ <url> | <string> ]
        //     [ layer | layer(<layer-name>) ]?
        //     <import-condition> ;

        // <import-conditions> = [ supports( [ <supports-condition> | <declaration> ] ) ]?
        //                      <media-query-list>?
        // @import url;
        // @import url layer;
        // @import url layer(layer-name);
        // @import url layer(layer-name) supports(supports-condition);
        // @import url layer(layer-name) supports(supports-condition) list-of-media-queries;
        // @import url layer(layer-name) list-of-media-queries;
        // @import url supports(supports-condition);
        // @import url supports(supports-condition) list-of-media-queries;
        // @import url list-of-media-queries;

        if !self.at(T![@import]) {
            return None
        }

        let m = self.start();
        self.bump_any();

        if self.parse_uri_literal_opt().is_none() && self.parse_string_literal().is_none() {
            self.err_and_bump_pe(ParseError::URIOrStringExpected);
        }

        Some(self.complete_parse_import(m))
    }

    pub fn complete_parse_import(&mut self, m: Marker) {
        // consume both `layer` and `layer(`
        if self.eat_contextual_token(T![cxfunc_layer]) {
            // TODO: check vscode css langservice accepts empty `layer()`
            if !self.at(SyntaxKind::R_PAREN) && self.parse_layer_name().is_none() {
                self.err_resync_pe(
                    ParseError::IdentifierExpected,
                    Some(TokenSet::new(&[T![;]])),
                    None,
                );
            }
            if !self.eat(SyntaxKind::R_PAREN) {
                self.err_resync_pe(
                    ParseError::RightParenthesisExpected,
                    Some(TokenSet::new(&[SyntaxKind::R_PAREN])),
                    None,
                );
            }
        }
        if self.eat_contextual_token(T![cxfunc_supports]) {
            self
                .try_parse_declaration_opt(None)
                .or_else(|| Some(self.parse_supports_condition()));

            if !self.eat(SyntaxKind::R_PAREN) {
                self.err_resync_pe(
                    ParseError::RightParenthesisExpected,
                    Some(TokenSet::new(&[SyntaxKind::R_PAREN])),
                    None,
                );
            }
        }
        if !matches!(self.current(), T![;] | SyntaxKind::EOF) {
            self.parse_media_query_list();
        }

        // self.eat_req_semicolon();

        self.varnish(m, SyntaxKind::IMPORT)
    }

    pub fn parse_namespace_opt(&mut self) -> Option<()> {
        // http://www.w3.org/TR/css3-namespace/
        // namespace  : NAMESPACE_SYM S* [IDENT S*]? [STRING|URI] S* ';' S*
        if !self.at(T![@namespace]) {
            return None
        }

        let m = self.start();
        self.bump_any();

        self.parse_ident_opt(None); // optional prefix


        if self.parse_uri_literal_opt().is_none() && self.parse_string_literal().is_none() {
            self.err_resync_pe(
                ParseError::URIExpected,
                Some(TokenSet::new(&[T![;]])),
                None,
            ); // TODO: parserror should be URIorStringLiteralExpected?
        }
        

        if !self.eat(T![;]) {
            self.err_pe(ParseError::SemiColonExpected);
        }
        Some(self.varnish(m, SyntaxKind::NAMESPACE))
    }

    pub fn parse_font_face_opt(&mut self) -> Option<()> {
        if !self.at(T![@font_face]) {
            return None
        }
        let m = self.start();
        self.bump_any();
        self.parse_body(|s: &mut Self| s.parse_rule_set_declaration_opt());
        Some(self.varnish(m, SyntaxKind::FONT_FACE))
    }

    pub fn parse_viewport_opt(&mut self) -> Option<()> {
        if !matches!(self.current(), T![@_ms_viewport]| T![@_o_viewport] | T![@viewport]) {
            return None
        }
        let m = self.start();
        self.bump_any();

        self.parse_body(|s: &mut Self| s.parse_rule_set_declaration_opt());
        Some(self.varnish(m, SyntaxKind::VIEW_PORT))
    }

    pub fn parse_keyframe_opt(&mut self) -> Option<()> {
        if !matches!(self.current(), T![@keyframes] | T![@_o_keyframes] | T![@_moz_keyframes] | T![@_webkit_keyframes]) {
            return None
        }

        let m = self.start();
        self.bump_any();

        if self.parse_keyframe_ident().is_none() {
            self.err_pe(ParseError::IdentifierExpected);
        }

        self.parse_body(|s: &mut Self| s.parse_keyframe_selector_opt().map(|_| SyntaxKind::KEYFRAME_SELECTOR));
        Some(self.varnish(m, SyntaxKind::KEYFRAME))
    }

    pub fn parse_keyframe_ident(&mut self) -> Option<()> {
        self.parse_ident_opt(Some(&[ReferenceType::Keyframe]))
    }

    pub fn parse_keyframe_selector_opt(&mut self) -> Option<()> {
        let m = self.start();

        let mut has_content = false;
        if self.parse_ident_opt(None).is_some() {
            has_content = true;
        }

        // if self.eat(T![%]) { // todo: only eat when !has_content ?
        //     has_content = true;
        // }
        // VSCode error?: probs does not err on 'from 0% {}' when it should
        if self.eat(T![DIM_PERCENT]) { // todo: only eat when !has_content?
            has_content = true;
        }

        if !has_content {
            m.rollback(self);
            return None
        }

        while self.eat(T![,]) {
            has_content = false;
            if self.parse_ident_opt(None).is_some() {
                has_content = true;
            }
            // VSCode error?: probs does not err on 'from 0% {}' when it should
            if self.eat(T![DIM_PERCENT]) { // todo: only eat when !has_content?
                has_content = true;
            }
            if !has_content {
                self.err_pe(ParseError::PercentageExpected) // TODO better error for keyframe selector expected 

            }
        }
        self.parse_body(|s: &mut Self| s.parse_rule_set_declaration_opt());
        Some(self.varnish(m, SyntaxKind::KEYFRAME_SELECTOR))
    }

    // pub fn parse_tryparse_keyframe_selector(&mut self) -> Option<()> {
    //     let node = self.orphan(CssNodeType::_BodyDeclaration(BodyDeclaration {
    //         declarations: None,
    //         body_decl_type: BodyDeclarationType::KeyframeSelector,
    //     }));

    //     let mark = self.mark();

    //     let mut has_content = false;
    //     if let Some(id) = self.parse_ident(None) {
    //         self.append(node, id);
    //         has_content = true;
    //     }
    //     if self.eat(SyntaxKind::Percentage) {
    //         has_content = true;
    //     }
    //     if !has_content {
    //         return None
    //     }
    //     while self.eat(SyntaxKind::Comma) {
    //         has_content = false;
    //         if let Some(id) = self.parse_ident(None) {
    //             self.append(node, id);
    //             has_content = true;
    //         }
    //         if self.eat(SyntaxKind::Percentage) {
    //             has_content = true;
    //         }
    //         if !has_content {
    //             self.restore_at_mark(mark);
    //             return None
    //         }
    //     }

    //     if !self.at(SyntaxKind::L_CURLY) {
    //         self.restore_at_mark(mark);
    //         return None
    //     }

    //     return self.parse_body(node, |s: &mut Self| s.parse_rule_set_declaration());
    // }

    pub fn parse_property_at_rule_opt(&mut self) -> Option<()> {
        // @property <custom-property-name> {
        // 	<declaration-list>
        //  }
        if !self.at(T![@property]) {
            return None
        }
        let m = self.start();
        self.bump_any();

        if !self.at_contextual_token(T![cxid_valid_custom_prop]) {
            self.err_pe(ParseError::IdentifierExpected);
        }

        if self.parse_ident_opt(Some(&[ReferenceType::Property])).is_none() {
            self.err_pe(ParseError::IdentifierExpected);
        }
        self.parse_body(|s: &mut Self| s.parse_declaration_opt(None).map(|_| SyntaxKind::DECLARATION_CUSTOM_PROPERTY));
        Some(self.varnish(m, SyntaxKind::PROPERTY_AT_RULE))
    }

    pub fn parse_layer_opt(&mut self, is_nested: bool) -> Option<()> {
        // @layer layer-name {rules}
        // @layer layer-name;
        // @layer layer-name, layer-name, layer-name;
        // @layer {rules}
        if !self.at(T![@layer]) {
            return None
        }
        let m = self.start();

        self.bump_any();

        let names = self.parse_layer_namelist_opt();
        
        if (names.is_none() || names.unwrap() == 1) && self.at(SyntaxKind::L_CURLY) {
            self.parse_body(|s: &mut Self| s.parse_layer_declaration(is_nested));
            return Some(self.varnish(m, SyntaxKind::LAYER))
        }
        if !self.eat(T![;]) {
            self.err_pe(ParseError::SemiColonExpected);
        }
        Some(self.varnish(m, SyntaxKind::LAYER))
    }

    pub fn parse_layer_declaration(&mut self, is_nested: bool) -> Option<SyntaxKind> {
        if is_nested {
            // if nested, the body can contain rulesets, but also declarations
            return self
                .parse_rule_set_opt(true).map(|_| SyntaxKind::RULE_SET)
                .or_else(|| self.try_parse_declaration_opt(None).map(|_| SyntaxKind::DECLARATION))
                .or_else(|| self.parse_stylesheet_statement_opt(true)) 
        }
        self.parse_stylesheet_statement_opt(false)
    }

    /// returns Option<number of names parsed>
    pub fn parse_layer_namelist_opt(&mut self) -> Option<usize> {
        let m = self.start();
        let mut name_count = 0_usize;
        if self.parse_layer_name().is_none() {
            m.rollback(self);
            return None
        }
        name_count += 1;

        while self.eat(T![,]) {
            if self.parse_layer_name().is_none() {
                self.err_pe(ParseError::IdentifierExpected); // TODO: more specific parse error
                return Some(name_count);
            }
            name_count += 1;
        }
        self.varnish(m, SyntaxKind::LAYER_NAME_LIST);
        Some(name_count)
    }

    pub fn parse_layer_name(&mut self) -> Option<()> {
        // <layer-name> = <ident> [ '.' <ident> ]*
        let m = self.start();
        if self.parse_ident_opt(None).is_none() {
            m.rollback(self);
            return None
        }
        while !self.has_whitespace() && self.eat(T![.]) {
            if self.has_whitespace() || self.parse_ident_opt(None).is_none() {
                self.err_pe(ParseError::IdentifierExpected);
            }
        }
        Some(self.varnish(m, SyntaxKind::LAYER_NAME))
    }

    pub fn parse_supports_opt(&mut self, is_nested: bool) -> Option<()> {
        // SUPPORTS_SYM S* supports_condition '{' S* ruleset* '}' S*
        if !self.at(T![@supports]) {
            return None
        }
        let m = self.start();
        self.bump_any();
        self.parse_supports_condition();
        self.parse_body(|s: &mut Self| s.parse_supports_declaration(is_nested));
        Some(self.varnish(m, SyntaxKind::SUPPORTS))
    }

    pub fn parse_supports_declaration(&mut self, is_nested: bool) -> Option<SyntaxKind> {
        if is_nested {
            // if nested, the body can contain rulesets, but also declarations
            return self
                .parse_rule_set_opt(true).map(|_| SyntaxKind::RULE_SET)
                .or_else(|| self.try_parse_declaration_opt(None).map(|_| SyntaxKind::DECLARATION))
                .or_else(|| self.parse_stylesheet_statement_opt(true));
        }
        self.parse_stylesheet_statement_opt(false)
    }

    pub fn parse_supports_condition(&mut self) {
        // TODO: check MDN syntax cfr. CSS spec
        // supports_condition : supports_negation | supports_conjunction | supports_disjunction | supports_condition_in_parens ;
        // supports_condition_in_parens: ( '(' S* supports_condition S* ')' ) | supports_declaration_condition | general_enclosed ;
        // supports_negation: NOT S+ supports_condition_in_parens ;
        // supports_conjunction: supports_condition_in_parens ( S+ AND S+ supports_condition_in_parens )+;
        // supports_disjunction: supports_condition_in_parens ( S+ OR S+ supports_condition_in_parens )+;
        // supports_declaration_condition: '(' S* declaration ')';
        // general_enclosed: ( FUNCTION | '(' ) ( any | unused )* ')' ;
        let m = self.start();

        // self.eat_contextual_token(T![cxid_not]);
        // self.parse_supports_condition_in_parens();

        // while self.eat_contextual_token(T![cxid_and]) || self.eat_contextual_token(T![cxid_or]) {
        //     self.parse_supports_condition_in_parens();
        // }

        
        if self.eat_contextual_token(T![cxid_not]) {
            self.parse_supports_condition_in_parens();
        } else {
            self.parse_supports_condition_in_parens();
            // TODO: weird code in VSCode CSS lang service: 
            // why peek case insensitive and|or and then
            // only accept lowercase? check spec
            if self.at_contextual_token(T![cxid_and]) {
                while self.eat_contextual_token(T![cxid_and]) {
                    self.parse_supports_condition_in_parens();
                }
            } else if self.at_contextual_token(T![cxid_or]) {
                while self.eat_contextual_token(T![cxid_or]) {
                    self.parse_supports_condition_in_parens();
                }
            }
        }
        
        m.complete(self, SyntaxKind::SUPPORTS_CONDITION);
    }

    pub fn parse_supports_condition_in_parens(&mut self) {
        // TODO: change function name and ending error
        let m = self.start();
        if self.eat(SyntaxKind::L_PAREN) {
            if self.try_parse_declaration_opt(Some(&[SyntaxKind::R_PAREN])).is_none() {
                self.parse_supports_condition(); 
                // TODO: Unreachable in VSCode return self.finito(m, ParseError::ConditionExpected);
            }

            if !self.eat(SyntaxKind::R_PAREN) {
                self.err_resync_pe(
                    ParseError::RightParenthesisExpected,
                    Some(TokenSet::new(&[SyntaxKind::R_PAREN])),
                    None,
                );
            }

            return self.varnish(m, SyntaxKind::SUPPORTS_CONDITION)
        } else if self.eat(T![function]) {
            let mut open_parent_count = 1;
            while open_parent_count != 0 {
                match self.current() {
                    SyntaxKind::EOF => break,
                    SyntaxKind::L_PAREN | T![function] => open_parent_count += 1,
                    SyntaxKind::R_PAREN => open_parent_count -= 1,
                    _ => {}
                }
                self.bump_any();
            }
            return self.varnish(m, SyntaxKind::SUPPORTS_CONDITION)
        }
        self.err_resync_pe(
            ParseError::LeftParenthesisExpected,
            Some(TokenSet::new(&[])),
            Some(TokenSet::new(&[SyntaxKind::L_PAREN])),
        );
        self.varnish(m, SyntaxKind::SUPPORTS_CONDITION);
    }

    pub fn parse_media_declaration(&mut self, is_nested: bool) -> Option<SyntaxKind> {
        if is_nested {
            // if nested, the body can contain rulesets, but also declarations
            return self
                .try_parse_rule_set_opt(true).map(|_| SyntaxKind::RULE_SET)
                .or_else(|| self.try_parse_declaration_opt(None).map(|_| SyntaxKind::RULE_SET))
                .or_else(|| self.parse_stylesheet_statement_opt(true));
        }
        self.parse_stylesheet_statement_opt(false)
    }

    pub fn parse_media_opt(&mut self, is_nested: bool) -> Option<()> {
        // MEDIA_SYM S* media_query_list '{' S* ruleset* '}' S*
        // media_query_list : S* [media_query [ ',' S* media_query ]* ]?
        if !self.at(T![@media]) {
            return None
        }
        let m = self.start();
        self.bump_any();
        self.parse_media_query_list();
        // if self.parse_media_query_list().is_err() {
        //     return Some(self.finito(m, ParseError::MediaQueryExpected))
        // }
        self.parse_body(|s: &mut Self| s.parse_media_declaration(is_nested));
        Some(self.varnish(m, SyntaxKind::MEDIA))
    }

    pub fn parse_media_query_list(&mut self) {
        let m = self.start();
        if self.parse_media_query_opt().is_none() {
            self.err_pe(ParseError::MediaQueryExpected);
        }
        while self.eat(T![,]) {
            if self.parse_media_query_opt().is_none() {
                self.err_pe(ParseError::MediaQueryExpected);
            }
        }
        self.varnish(m, SyntaxKind::UNDEFINED)
    }

    pub fn parse_media_query_opt(&mut self) -> Option<()> {
        // <media-query> = <media-condition> | [ not | only ]? <media-type> [ and <media-condition-without-or> ]?
        let m = self.start();
        //let mark = self.mark();
        self.eat_contextual_token(T![cxid_not]);
        if !self.at(SyntaxKind::L_PAREN) {
            if self.eat_contextual_token(T![cxid_only]) {
                // optional
            }
            if self.parse_ident_opt(None).is_none() {
                m.rollback(self);
                return None
            };
            if self.eat_contextual_token(T![cxid_and]) {
                self.parse_media_condition();
            }
        } else {
            //self.restore_at_mark(mark);
            self.parse_media_condition();
        }
        Some(self.varnish(m, SyntaxKind::MEDIA_QUERY))
    }

    pub fn parse_ratio_opt(&mut self) -> Option<()> {
        let m = self.start();
        if self.parse_numeric().is_none() || !self.eat(T![/]) {
            m.rollback(self);
            return None
        }
        if self.parse_numeric().is_none() {
            self.err_pe(ParseError::NumberExpected);
        }
        Some(self.varnish(m, SyntaxKind::RATIO_VALUE))
    }

    pub fn parse_media_condition(&mut self) {
        // <media-condition> = <media-not> | <media-and> | <media-or> | <media-in-parens>
        // <media-not> = not <media-in-parens>
        // <media-and> = <media-in-parens> [ and <media-in-parens> ]+
        // <media-or> = <media-in-parens> [ or <media-in-parens> ]+
        // <media-in-parens> = ( <media-condition> ) | <media-feature> | <general-enclosed>

        let m = self.start();
        self.eat_contextual_token(T![cxid_not]);
        let mut parse_expression = true;

        while parse_expression {
            if !self.eat(SyntaxKind::L_PAREN) {
                self.err_resync_pe(
                    ParseError::LeftParenthesisExpected,
                    None,
                    Some(TokenSet::new(&[SyntaxKind::L_CURLY])),
                );
            }
            if self.at(SyntaxKind::L_PAREN) || self.at_contextual_token(T![cxid_not]) {
                // <media-condition>
                self.parse_media_condition();
            } else {
                self.parse_media_feature();
            }
            // not yet implemented: general enclosed    <TODO?>
            if !self.eat(SyntaxKind::R_PAREN) {
                self.err_pe(ParseError::RightParenthesisExpected);
            }
            parse_expression = self.eat_contextual_token(T![cxid_and]) || self.eat_contextual_token(T![cxid_or]);
        }
        self.varnish(m,SyntaxKind::MEDIA_CONDITION)
    }

    pub fn parse_media_feature(&mut self) {
        let resync_stop_token = Some(TokenSet::new(&[SyntaxKind::R_PAREN]));

        let m = self.start();
        // <media-feature> = ( [ <mf-plain> | <mf-boolean> | <mf-range> ] )
        // <mf-plain> = <mf-name> : <mf-value>
        // <mf-boolean> = <mf-name>
        // <mf-range> = <mf-name> [ '<' | '>' ]? '='? <mf-value> | <mf-value> [ '<' | '>' ]? '='? <mf-name> | <mf-value> '<' '='? <mf-name> '<' '='? <mf-value> | <mf-value> '>' '='? <mf-name> '>' '='? <mf-value>

        if self.parse_media_feature_name().is_some() {
            if self.eat(T![:]) {
                if self.parse_media_feature_value_opt().is_none() {
                    self.err_resync_pe(
                        ParseError::TermExpected,
                        None,
                        resync_stop_token,
                    );
                }
            } else if self.parse_media_feature_range_operator() {
                if self.parse_media_feature_value_opt().is_none() {
                    self.err_resync_pe(
                        ParseError::TermExpected,
                        None,
                        resync_stop_token,
                    );
                } else if self.parse_media_feature_range_operator() && self.parse_media_feature_value_opt().is_none() {
                    self.err_resync_pe(
                        ParseError::TermExpected,
                        None,
                        resync_stop_token,
                    );
                }
            } else {
                // <mf-boolean> = <mf-name>
            }
        } else if self.parse_media_feature_value_opt().is_some() {
            if !self.parse_media_feature_range_operator() {
                self.err_resync_pe(
                    ParseError::OperatorExpected,
                    None,
                    resync_stop_token,
                );
            } else if self.parse_media_feature_name().is_none() {
                self.err_resync_pe(
                    ParseError::IdentifierExpected,
                    None,
                    resync_stop_token,
                );
            } else if self.parse_media_feature_range_operator() && self.parse_media_feature_value_opt().is_none() {
                self.err_resync_pe(
                    ParseError::TermExpected,
                    None,
                    resync_stop_token,
                );
            }
        } else {
            self.err_resync_pe(
                ParseError::IdentifierExpected,
                None,
                resync_stop_token,
            );
        }
        self.varnish(m, SyntaxKind::MEDIA_FEATURE)
    }

    pub fn parse_media_feature_range_operator(&mut self) -> bool {
        if self.eat(T![<]) || self.eat(T![>]) {
            if !self.has_whitespace() {
                self.eat(T![=]);
            }
            return true;
        } else if self.eat(T![=]) {
            return true;
        }
        false
    }

    pub fn parse_media_feature_name(&mut self) -> Option<()> {
        self.parse_ident_opt(None)
    }

    pub fn parse_media_feature_value_opt(&mut self) -> Option<()> {
        self.parse_ratio_opt().map(|_| ()).or_else(|| self.parse_term_expression_opt())
    }

    // pub fn parse_medium_opt(&mut self) -> Option<()> {
    //     let m = self.start();
    //     if self.parse_ident_opt(None).is_none() {
    //         m.rollback(self);
    //         return None
    //     }
    //     Some(self.varnish(m, SyntaxKind::UNDEFINED))
    // }

    pub fn parse_page_declaration(&mut self) -> Option<SyntaxKind> {
        self
            .parse_page_margin_box().map(|_| SyntaxKind::PAGE_BOX_MARGIN_BOX)
            .or_else(|| self.parse_rule_set_declaration_opt())
    }

    pub fn parse_page(&mut self) -> Option<()> {
        // http://www.w3.org/TR/css3-page/
        // page_rule : PAGE_SYM S* page_selector_list '{' S* page_body '}' S*
        // page_body :  /* Can be empty */ declaration? [ ';' S* page_body ]? | page_margin_box page_body
        if !self.at(T![@page]) {
            return None
        }
        let m = self.start();
        self.bump_any();
        if self.parse_page_selector_opt().is_some() {
            while self.eat(T![,]) {
                if self.parse_page_selector_opt().is_none() {
                    self.err_pe(ParseError::IdentifierExpected);
                }
            }
        }
        self.parse_body(|s: &mut Self| s.parse_page_declaration());
        Some(self.varnish(m, SyntaxKind::PAGE))
    }

    pub fn parse_page_margin_box(&mut self) -> Option<()> {
        // page_margin_box :  margin_sym S* '{' S* declaration? [ ';' S* declaration? ]* '}' S*
        if !self.current().is_at_keyword() {
            return None
        }
        let m = self.start();
        if !self.eat(T![@margin_at_rule]) {
            self.err_resync_pe(
                ParseError::UnknownAtRule,
                Some(TokenSet::new(&[])),
                Some(TokenSet::new(&[SyntaxKind::L_CURLY])),
            );
        }
        self.parse_body(|s: &mut Self| s.parse_rule_set_declaration_opt());
        Some(self.varnish(m, SyntaxKind::PAGE_BOX_MARGIN_BOX))
    }

    pub fn parse_page_selector_opt(&mut self) -> Option<()> {
        // page_selector : pseudo_page+ | IDENT pseudo_page*
        // pseudo_page :  ':' [ "left" | "right" | "first" | "blank" ];
        if !self.at(T![identifier]) && !self.at(T![:]) {
            return None
        }
        let m = self.start();
        self.parse_ident_opt(None); // optional ident
        if self.eat(T![:]) && self.parse_ident_opt(None).is_none() {
            self.err_pe(ParseError::IdentifierExpected);
        }
        Some(self.varnish(m, SyntaxKind::UNDEFINED))
    }

    pub fn parse_document_opt(&mut self) -> Option<()> {
        // -moz-document is experimental but has been pushed to css4
        if !self.at(T![@_moz_document]) {
            return None
        }
        let m = self.start();
        self.bump_any();
        while self.current() != SyntaxKind::L_CURLY {
            // ignore all rules
            self.bump_any();
        }
        self.parse_body(|s: &mut Self| s.parse_stylesheet_statement_opt(false));
        Some(self.varnish(m, SyntaxKind::DOCUMENT))
    }

    pub fn parse_container(&mut self) -> Option<()> {
        if !self.at(T![@container]) {
            return None
        }
        let m = self.start();
        self.bump_any();
        self.parse_ident_opt(None);
        self.parse_container_query();
        self.parse_body(|s: &mut Self| s.parse_stylesheet_statement_opt(false));
        Some(self.varnish(m, SyntaxKind::CONTAINER))
    }

    pub fn parse_container_query(&mut self) {
        // <container-query>     = not <query-in-parens>
        //                         | <query-in-parens> [ [ and <query-in-parens> ]* | [ or <query-in-parens> ]* ]
        let m = self.start();
        self.parse_container_query_in_parens();
        if !self.eat_contextual_token(T![cxid_not]) {
            if self.at_contextual_token(T![cxid_and]) {
                while self.eat_contextual_token(T![cxid_and]) {
                    self.parse_container_query_in_parens();
                }
            } else if self.at_contextual_token(T![cxid_or]) {
                while self.eat_contextual_token(T![cxid_or]) {
                    self.parse_container_query_in_parens();
                }
            }
        }
        self.varnish(m, SyntaxKind::UNDEFINED)
    }

    pub fn parse_container_query_in_parens(&mut self) {
        // <query-in-parens>     = ( <container-query> )
        // 					  | ( <size-feature> )
        // 					  | style( <style-query> )
        // 					  | <general-enclosed>
        let m = self.start();
        if self.eat(SyntaxKind::L_PAREN) {
            if self.at_contextual_token(T![cxid_not]) || self.at(SyntaxKind::L_PAREN) {
                self.parse_container_query();
            } else { 
                self.parse_media_feature();
            }
            if !self.eat(SyntaxKind::R_PAREN) {
                self.err_resync_pe(
                    ParseError::RightParenthesisExpected,
                    None,
                    Some(TokenSet::new(&[SyntaxKind::L_CURLY])),
                );
            }
        } else if self.eat_contextual_token(T![cxfunc_style]) {
            // if self.has_whitespace() || !self.eat(SyntaxKind::L_PAREN) {
            //     return self.fintio_recover(
            //         m,
            //         ParseError::LeftParenthesisExpected,
            //         None,
            //         Some(&[SyntaxKind::L_CURLY]),
            //     );
            // }
            self.parse_style_query();
            if !self.eat(SyntaxKind::R_PAREN) {
                self.err_resync_pe(
                    ParseError::RightParenthesisExpected,
                    None,
                    Some(TokenSet::new(&[SyntaxKind::L_CURLY])),
                );
            }
        } else {
            self.err_resync_pe(
                ParseError::LeftParenthesisExpected,
                None,
                Some(TokenSet::new(&[SyntaxKind::L_CURLY])),
            );
        }
        self.varnish(m, SyntaxKind::UNDEFINED)
    }

    pub fn parse_style_query(&mut self) {
        // <style-query>         = not <style-in-parens>
        // 					  | <style-in-parens> [ [ and <style-in-parens> ]* | [ or <style-in-parens> ]* ]
        // 					  | <style-feature>
        // <style-in-parens>     = ( <style-query> )
        // 					  | ( <style-feature> )
        // 					  | <general-enclosed>
        let m = self.start();
        if self.eat_contextual_token(T![cxid_not]) {
            self.parse_style_in_parens();
        } else if self.at(SyntaxKind::L_PAREN) {
            self.parse_style_in_parens();
            if self.at_contextual_token(T![cxid_and]) {
                while self.eat_contextual_token(T![cxid_and]) {
                    self.parse_style_in_parens();
                }
            } else if self.at_contextual_token(T![cxid_or]) {
                while self.eat_contextual_token(T![cxid_or]) {
                    self.parse_style_in_parens();
                }
            }
        } else {
            self.parse_declaration_opt(Some(&[SyntaxKind::R_PAREN]));
        }
        self.varnish(m, SyntaxKind::UNDEFINED)
    }

    pub fn parse_style_in_parens(&mut self) {
        let m = self.start();
        if self.eat(SyntaxKind::L_PAREN) {
            self.parse_style_query();
            if !self.eat(SyntaxKind::R_PAREN) {
                self.err_resync_pe(
                    ParseError::RightParenthesisExpected,
                    None,
                    Some(TokenSet::new(&[SyntaxKind::L_CURLY])),
                );
            }
        } else {
            self.err_resync_pe(
                ParseError::LeftParenthesisExpected,
                None,
                Some(TokenSet::new(&[SyntaxKind::L_CURLY])),
            );
        }
        self.varnish(m, SyntaxKind::UNDEFINED)
    }

    // https://www.w3.org/TR/css-syntax-3/#consume-an-at-rule
    pub fn parse_unknown_at_rule(&mut self) -> Option<()> {
        if !self.current().is_at_keyword() {
            return None
        }
        let m = self.start();
        self.parse_unknown_at_rule_name();

        let mut curly_l_count = 0;
        let mut curly_dep = 0;
        let mut parens_dep = 0;
        let mut bracks_dep = 0;
        macro_rules! is_top_lvl {
            () => {
                curly_dep == 0 && parens_dep == 0 && bracks_dep == 0
            };
        }

        loop {
            match self.current() {
                T![;] => {
                    if is_top_lvl!() {
                        break;
                    }
                }
                SyntaxKind::EOF => {
                    if curly_dep > 0 {
                        self.err_pe(
                            ParseError::RightCurlyExpected
                        );
                    } else if bracks_dep > 0 {
                        self.err_pe(
                            ParseError::RightSquareBracketExpected
                        );
                    } else if parens_dep > 0 {
                        self.err_pe(
                            ParseError::RightParenthesisExpected
                        );
                    }
                    return Some(self.varnish(m, SyntaxKind::UNKNOWN_AT_RULE))
                }
                SyntaxKind::L_CURLY => {
                    curly_l_count += 1;
                    curly_dep += 1;
                }
                SyntaxKind::R_CURLY => {
                    curly_dep -= 1;
                    // end of at-rule, consume R_CURLY and return node
                    if curly_l_count > 0 && curly_dep == 0 {
                        self.bump(SyntaxKind::R_CURLY);
                        if bracks_dep > 0 {
                            self.err_pe(
                                ParseError::RightSquareBracketExpected
                            );
                            break;
                        } else if parens_dep > 0 {
                            Some(self.err_pe(
                                ParseError::RightParenthesisExpected
                            ));
                            break;
                        }
                        break;
                    }
                    if curly_dep < 0 {
                        // the property value has been terminated without a semicolon,
                        // and this is the last declaration in the ruleset
                        if parens_dep == 0 && bracks_dep == 0 {
                            break;
                        }
                        self.err_pe(
                            ParseError::LeftCurlyExpected
                        );
                        break;
                    }
                }
                SyntaxKind::L_PAREN |
                T![function] => {
                    parens_dep += 1;
                }
                SyntaxKind::R_PAREN => {
                    parens_dep -= 1;
                    if parens_dep < 0 {
                        self.err_pe(
                            ParseError::LeftParenthesisExpected
                        );
                        break;
                    }
                }
                SyntaxKind::L_BRACK => {
                    bracks_dep += 1;
                }
                SyntaxKind::R_BRACK => {
                    bracks_dep -= 1;
                    if bracks_dep < 0 {
                        self.err_pe(
                            ParseError::LeftSquareBracketExpected
                        );
                    }
                }
                _ => {}
            }
            self.bump_any();
        }
        Some(self.varnish(m, SyntaxKind::UNKNOWN_AT_RULE))
    }

    pub fn parse_unknown_at_rule_name(&mut self) -> Option<()> {
        if !self.eat(T![@unknown]) {
            return None
        }
        Some(())
    }

    pub fn parse_operator_opt(&mut self) -> Option<()> {
        let m = self.start();

        let mut make_sel_operator = |sk: SyntaxKind| (self.eat(sk)).then_some(());

        if  make_sel_operator(SyntaxKind::OPERATOR_DASHMATCH).or_else(||
            make_sel_operator(SyntaxKind::OPERATOR_INCLUDES)).or_else(||
            make_sel_operator(SyntaxKind::OPERATOR_SUBSTRING)).or_else(||
            make_sel_operator(SyntaxKind::OPERATOR_PREFIX)).or_else(||
            make_sel_operator(SyntaxKind::OPERATOR_SUFFIX)).is_some()
        {
            self.varnish(m, SyntaxKind::OPERATOR);
            return Some(())
        }

        // operators for binary expression
        const SINGLE_SYMBOL_OPERATOR_TOKENSET: TokenSet = TokenSet::new(&[T![/], T![+], T![-], T![=], T![*]]); 
        if !self.at_ts(SINGLE_SYMBOL_OPERATOR_TOKENSET) {
            m.rollback(self);
            return None
        }
        self.bump_any();
        self.varnish(m, SyntaxKind::OPERATOR);
        Some(())
    }

    pub fn parse_unary_operator_opt(&mut self) -> Option<()> {
        const UNARY_OPERATORS: TokenSet = TokenSet::new(&[T![+], T![-]]);
        if !self.at_ts(UNARY_OPERATORS) {
            return None
        }
        let m = self.start();
        self.bump_any();
        Some(self.varnish(m, SyntaxKind::OPERATOR))
    }

    pub fn parse_combinator_opt(&mut self) -> Option<()> {
        let m = self.start();
        if self.eat(T![>]) {
            let sk = if !self.has_whitespace() && !self.has_n_whitespace(1) && self.at(T![>]) && self.nth_at(1, T![>]) {
                self.bump_any();
                self.bump_any();
                SyntaxKind::SELECTOR_COMBINATOR_SHADOW_PIERCING_DESCENDANT
            } else {
                SyntaxKind::SELECTOR_COMBINATOR_PARENT
            };
            self.varnish(m, sk);
            return Some(())
        } else if self.eat(T![+]) {
            self.varnish(m, SyntaxKind::SELECTOR_COMBINATOR_SIBLING);
            return Some(())
        } else if self.eat(T![~]) {
            self.varnish(m, SyntaxKind::SELECTOR_COMBINATOR_ALL_SIBLINGS);
            return Some(())
        } else if self.eat(T![/])
            && !self.has_whitespace()
            && self.eat_contextual_token(T![cxid_deep])
            && !self.has_whitespace()
            && self.at(T![/])
        {
            self.varnish(m, SyntaxKind::SELECTOR_COMBINATOR_SHADOW_PIERCING_DESCENDANT);
            return Some(())
        }
        m.rollback(self);
        None
    }

    pub fn parse_simple_selector(&mut self) -> Option<()> {
        // simple_selector
        //  : element_name [ HASH | class | attrib | pseudo ]* | [ HASH | class | attrib | pseudo ]+ ;
        let m = self.start();
        let mut c = 0;
        if self
            .parse_element_name()
            .or_else(|| self.parse_nesting_selector_opt())
            .is_some()
        {
            c += 1;
        }

        while (c == 0 || !self.has_whitespace()) && self.parse_simple_selector_body().is_some() {
            c += 1;
        }
        if c == 0 {
            m.rollback(self);
            return None
        }
        self.varnish(m, SyntaxKind::SIMPLE_SELECTOR);
        Some(())
    }

    pub fn parse_nesting_selector_opt(&mut self) -> Option<()> {
        if !self.at(T![&]) {
            return None
        }
        let m = self.start();
        self.bump_any();
        self.varnish(m, SyntaxKind::SELECTOR_COMBINATOR); // TODO NESTING SELECTOR
        Some(())
    }

    pub fn parse_simple_selector_body(&mut self) -> Option<()> {
        self
            .parse_pseudo_opt()
            .or_else(|| self.parse_selector_identifier_opt())
            .or_else(|| self.parse_class_opt())
            .or_else(|| self.parse_attribute())
    }

    pub fn parse_selector_ident(&mut self) -> Option<()> {
        self.parse_ident_opt(None)
    }

    pub fn parse_selector_identifier_opt(&mut self) -> Option<()> {
        // err on unrestricted hash or T![#] with no followup
        if !self.at(T![id_hash]) {
            return None
        }
        let m = self.start();
        self.bump_any();
        Some(self.varnish(m, SyntaxKind::SELECTOR_IDENTIFIER))
    }

    pub fn parse_class_opt(&mut self) -> Option<()> {
        // `.IDENT`
        if !self.at(T![.]) {
            return None
        }
        let m = self.start();
        self.bump_any(); // `.`
        if self.has_whitespace() || self.parse_selector_ident().is_none() {
            self.err_pe(ParseError::IdentifierExpected);
        }
        Some(self.varnish(m, SyntaxKind::SELECTOR_CLASS))
    }

    pub fn parse_element_name(&mut self) -> Option<()> {
        // (namespace? `|`)? IDENT | `*`
        let m = self.start();
        self.parse_namespace_prefix();
        if self.parse_selector_ident().is_none() && !self.eat(T![*]) {
            m.rollback(self);
            return None
        }
        self.varnish(m, SyntaxKind::SELECTOR_ELEMENT_NAME);
        Some(())
    }

    pub fn parse_namespace_prefix(&mut self) -> Option<()> {
        let m = self.start();
        if self.parse_ident_opt(None).is_none() && !self.eat(T![*]) {
            // namespace is optional
        }
        // don't match if at selector attribute operator
        if self.at(T![|=]) || !self.eat(T![|]) {
            m.rollback(self);
            return None
        }
        self.varnish(m, SyntaxKind::NAMESPACE_PREFIX);
        Some(())
    }

    pub fn parse_attribute(&mut self) -> Option<()> {
        // attrib : '[' S* IDENT S* [ [ '=' | INCLUDES | DASHMATCH ] S*   [ IDENT | STRING ] S* ]? ']'
        if !self.at(SyntaxKind::L_BRACK) {
            return None
        }

        let m = self.start();
        self.bump_any(); // `[`

        self.parse_namespace_prefix(); // optional attribute namespace

        if self.parse_ident_opt(None).is_none() {
            self.err_pe(ParseError::IdentifierExpected);
        }

        if self.parse_operator_opt().is_some() {
            self.parse_binary_expr();
            self.eat_contextual_token(T![cxid_attrib_i]); // case insensitive matching e.g. `a[href$=".org" i] ``
            self.eat_contextual_token(T![cxid_attrib_s]); // case sensitive matching
        }

        if !self.eat(SyntaxKind::R_BRACK) {
            self.err_pe(
                ParseError::RightSquareBracketExpected
            );
        }
        Some(self.varnish(m, SyntaxKind::SELECTOR_ATTRIBUTE))
    }

    pub fn parse_pseudo_opt(&mut self) -> Option<()> {
        // ':' [ IDENT | FUNCTION S* [IDENT S*]? ')' ]
        let m = self.start();

        let Some(is_func) = self.try_parse_pseudo_identifier_and_is_func() else {
            m.rollback(self);
            return None
        };

        if is_func.is_some_and(|is_func| is_func) {

            let try_as_selector = |s: &mut Self| -> Option<()> {
                let selectors = s.start();
                if s.parse_selector_opt(true).is_none() {
                    selectors.rollback(s);
                    return None
                }
                while s.eat(T![,]) && s.parse_selector_opt(true).is_some() {
                    // loop
                }
                if !s.at(SyntaxKind::R_PAREN) {
                    selectors.rollback(s);
                    return None
                }
                s.varnish(selectors, SyntaxKind::UNDEFINED);
                Some(())
            };

            if self.eat_contextual_token(SyntaxKind::CXDIM_AN_PLUS_B) || self.eat_contextual_token(SyntaxKind::CXID_AN_PLUS_B_SYNTAX_AN) {
                if !self.eat(T![number]) && self.eat(T![+]) {
                    self.eat(T![number]);
                };
                if self.eat_contextual_token(T![cxid_of]) && try_as_selector(self).is_none() {
                    self.err_pe(ParseError::SelectorExpected);
                }
                if !self.eat(SyntaxKind::R_PAREN) {
                    self.err_pe(ParseError::RightParenthesisExpected);
                }
                return Some(self.varnish(m, SyntaxKind::SELECTOR_PSEUDO))
            }

            let has_selector = try_as_selector(self).is_some();

            if !has_selector && self.parse_binary_expr().is_some() && 
                self.eat_contextual_token(T![cxid_of]) && try_as_selector(self).is_none() 
            {
                self.err_pe(ParseError::SelectorExpected);
            }

            if !self.eat(SyntaxKind::R_PAREN) {
                self.err_pe(ParseError::RightParenthesisExpected);
            }
        }
        Some(self.varnish(m, SyntaxKind::SELECTOR_PSEUDO))
    }

    pub fn try_parse_pseudo_identifier_and_is_func(&mut self) -> Option<Option<bool>> {
        if !self.at(T![:]) {
            return None
        }
        let m = self.start();
        self.bump_any(); // ':'
        if self.has_whitespace() {
            m.rollback(self);
            return None
        }
        self.eat(T![:]); // support ::
        if self.has_whitespace() {
            self.err_pe(ParseError::IdentifierExpected); // TODO: better error: pseudo selector expected
            m.abandon(self);
            return Some(None)
        }
        let ident = self.parse_ident_opt(None).is_some();
        if !ident && !self.eat(T![function]) {
            self.err_pe(ParseError::IdentifierExpected);
            m.abandon(self);
            return Some(None)
        }
        m.abandon(self);
        Some(Some(!ident))
    }

    // pub fn try_parse_prio(&mut self) -> Option<()> {
    //     let m = self.start();
    //     if self.parse_prio().is_some() {
    //         self.varnish(m, SyntaxKind::TODO);
    //         return Some(())
    //     }
    //     m.rollback(self);
    //     return None
    // }

    pub fn parse_prio_opt(&mut self) -> Option<()> {
        let m = self.start();
        if !self.eat(T![!]) || !self.eat_contextual_token(T![cxid_important]) {
            m.rollback(self);
            return None
        }
        self.varnish(m, SyntaxKind::PRIO);
        Some(())
    }

    pub fn parse_expr_opt(&mut self, stop_on_comma: bool) -> Option<()> {
        let m = self.start();
        if self.parse_binary_expr().is_none() {
            m.rollback(self);
            return None
        }
        loop {
            if self.at(T![,]) {
                // optional
                if stop_on_comma {
                    break
                }
                self.bump_any();
            }
            if self.parse_binary_expr().is_none() {
                break;
            }
        }
        self.varnish(m, SyntaxKind::EXPRESSION);
        Some(())
    }

    pub fn parse_unicode_range(&mut self) -> Option<()> {
        if !self.eat(T![unicode_range]) {
            return None
        }
        Some(())
    }

    pub fn parse_named_line(&mut self) -> Option<()> {
        // https://www.w3.org/TR/css-grid-1/#named-lines
        if !self.at(SyntaxKind::L_BRACK) {
            return None
        }
        let m = self.start();
        self.bump_any(); // [
        while self.parse_ident_opt(None).is_some() {
            // loop
        }
        if !self.eat(SyntaxKind::R_BRACK) {
            self.err_pe(ParseError::RightSquareBracketExpected);
        }
        Some(self.varnish(m, SyntaxKind::GRID_LINE))
    }

    pub fn parse_binary_expr(&mut self) -> Option<()> {
        self.parse_binary_expr_internal(false)
    }

    fn parse_binary_expr_internal(
        &mut self,
        preparsed_left_and_oper: bool,
    ) -> Option<()> {
        let m = self.start();

        if !preparsed_left_and_oper {
            if self.parse_term().is_none() {
                m.rollback(self);
                return None
            }
            if self.parse_operator_opt().is_none() {
                return Some(self.varnish(m, SyntaxKind::BINARY_EXPRESSION))
            }
        }
    
        if self.parse_term().is_none() {
            self.err_pe(ParseError::TermExpected);
        }

        //  multiple binary expressions
        // todo: add SyntaxNode::BinaryExpressionNested with just children operator & right (no left, because left is parent)
        let sec = self.start();
        if self.parse_operator_opt().is_some() {
            self.parse_binary_expr_internal(true);
            sec.complete(self, SyntaxKind::BINARY_EXPRESSION);
        } else {
            sec.rollback(self);
        }

        Some(self.varnish(m, SyntaxKind::BINARY_EXPRESSION))
    }

    pub fn parse_term(&mut self) -> Option<()> {
        let m = self.start();
        // optional
        self.parse_unary_operator_opt();

        if self.parse_term_expression_opt().is_none() {
            m.rollback(self);
            return None
        }
        self.varnish(m, SyntaxKind::TERM);
        Some(())
    }

    pub fn parse_term_expression_opt(&mut self) -> Option<()> {
        self
            .parse_uri_literal_opt().map(|_| ()) // url before function
            .or_else(|| self.parse_unicode_range())
            .or_else(|| self.parse_function_with_args_opt().map(|_| ())) // function before ident
            .or_else(|| self.parse_ident_opt(None))
            .or_else(|| self.parse_string_literal())
            .or_else(|| self.parse_numeric())
            .or_else(|| self.parse_hex_color_opt())
            .or_else(|| self.parse_operation().map(|_| ()))
            .or_else(|| self.parse_named_line().map(|_| ()))
    }

    pub fn parse_operation(&mut self) -> Option<()> {
        if !self.at(SyntaxKind::L_PAREN) {
            return None
        }
        let m = self.start();
        self.bump_any(); // '('
        self.parse_expr_opt(false);
        if !self.eat(SyntaxKind::R_PAREN) {
            self.err_pe(ParseError::RightParenthesisExpected);
        }
        Some(self.varnish(m, SyntaxKind::UNDEFINED))
    }

    pub fn parse_numeric(&mut self) -> Option<()> {
        if !self.at(T![number])
            && !self.current().is_dimension()
        {
           return None
        }
        let m = self.start();
        self.bump_any();
        self.varnish(m, SyntaxKind::NUMERIC_VALUE);
        Some(())
    }

    pub fn parse_string_literal(&mut self) -> Option<()> {
        if !self.eat(T![string]) && !self.eat(T![bad_string]) {
            return None
        }
        Some(())
        // if !self.at(T![string]) && !self.at(T![bad_string]) {
        //     return None
        // }
        // let m = self.start();
        // self.bump_any();
        // self.varnish(m, SyntaxKind::STRING_LITERAL);
    }

    pub fn parse_uri_literal_opt(&mut self) -> Option<()> {
        let m = self.start();
        if !self.eat(T![url]) && !self.eat(T![bad_url]) {
            if !self.at_contextual_token(T![cxfunc_url]) || !matches!(self.nth(1), T![string] | T![bad_string]) {
                m.rollback(self);
                return None
            }
            self.bump_any(); // url(
            self.bump_any(); // string / badstring
            if !self.eat(SyntaxKind::R_PAREN) {
                self.err_pe(ParseError::RightParenthesisExpected);
            }
        }
        Some(self.varnish(m, SyntaxKind::URI_LITERAL))
        
        // if !self.peek_regex(
        //     T![identifier],
        //     RegexBuilder::new("^url(-prefix)?$")
        //         .case_insensitive(true)
        //         .build()
        //         .unwrap(),
        // ) {
        //     return None
        // }
        // if !matches!(self.current(), T![cx_url] | T![cx_urlprefix]) {
        //     return None
        // }
        // let m = self.start();
        // self.bump_any();
        // if self.has_whitespace() || !self.at(SyntaxKind::L_PAREN) {
        //     m.rollback(self);
        //     return None
        // }
        // // TODO self.scanner.in_url = true;
        // // prob fix this in tokenizer?
        // self.bump_any(); // '()'
        // self.parse_url_argument(); // optional
        // //self.scanner.in_url = false;
        // if !self.eat(SyntaxKind::R_PAREN) {
        //     return Some(self.finito(m, ParseError::RightParenthesisExpected))
        // }

        // Some(self.varnish(m, SyntaxKind::URI_LITERAL))
    }

    // pub fn parse_url_argument(&mut self) -> Option<()> {
    //     // let m = self.start();
    //     // // if !self.eat(T!!("sk string"))
    //     // //     && !self.eat(T!("sk badstring"))
    //     // //     && !self.accept_unquoted_string()
    //     // // {
    //     // //     m.rollback(self);
    //     // //     return None
    //     // // }

    //     // self.varnish(m, SyntaxKind::DUNNO);
    //     // Some(())
    //     if !self.eat(T![url]) && !self.eat(T![bad_url]) {
    //         return None
    //     }
    //     Some(())
    // }

    pub fn parse_ident_opt(&mut self, _reference_types: Option<&[ReferenceType]>) -> Option<()> {
        // TODO reference type
        if !self.eat(T![identifier]) {
            return None
        }
        Some(())
        // if !self.at(T![identifier]) {
        //     return None
        // }
        // let m = self.start();
        // self.bump_any();
        // self.varnish(m, T![identifier]); 
        // Some(())
    }

    pub fn parse_function_with_args_opt(&mut self) -> Option<()> {

        if !self.at(T![function]) {
            return None
        }

        let m = self.start(); 
        self.bump_any();

        if self.parse_function_argument().is_some() {
            while self.eat(T![,]) {
                if self.at(SyntaxKind::R_PAREN) {
                    break;
                }
                if self.parse_function_argument().is_none() {
                    self.error(ParseError::ExpressionExpected.issue().desc);
                }
            }
        }

        if !self.eat(SyntaxKind::R_PAREN) {
            self.err_pe(ParseError::RightParenthesisExpected);
        }


        Some(self.varnish(m, SyntaxKind::FUNCTION_WITH_ARGS))
    }

    pub fn parse_function_argument(&mut self) -> Option<()> {
        let m = self.start();
        if self.parse_expr_opt(true).is_none() {
            m.rollback(self);
            return None
        }
        self.varnish(m, SyntaxKind::FUNCTION_ARGUMENT);
        Some(())
    }

    pub fn parse_hex_color_opt(&mut self) -> Option<()> {
        if self.at_contextual_token(T![cxhash_valid_hex]) {
            let m = self.start();
            self.bump_any();
            self.varnish(m, SyntaxKind::HEX_COLOR_VALUE);
            return Some(())
        }
        None
    }

}