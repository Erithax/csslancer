#![allow(clippy::needless_return)]

use proc_macro_error::abort_call_site;

use proc_macro2::{Ident, TokenStream, TokenTree};
use proc_macro_error::{abort, proc_macro_error};
use quote::{quote, ToTokens};

struct Parts {
    id: Ident,
    args: Option<TokenStream>,
    then_block: Option<TokenStream>,
    else_block: Option<TokenStream>,
    fun: Ident,
}

fn addchild_internal(input: proc_macro::TokenStream) -> Parts {
    let input = proc_macro2::TokenStream::from(input);

    let mut tokens = input.into_iter();

    let id = match tokens.next() {
        None => {
            abort_call_site!("expected identifier")
        }
        Some(t) => match t {
            TokenTree::Ident(i) => i,
            TokenTree::Group(g) => {
                abort!(g, "expected identifier");
            }
            TokenTree::Punct(p) => {
                abort!(p, "expected identifier");
            }
            TokenTree::Literal(l) => {
                abort!(l, "expected identifier");
            }
        },
    };

    fn try_args(t: &TokenTree) -> Option<TokenStream> {
        match t {
            TokenTree::Group(g) => {
                if g.delimiter() != proc_macro2::Delimiter::Parenthesis {
                    abort!(
                        g,
                        "expected function args in parentheses, or `then` or `else`"
                    );
                }
                Some(g.stream())
            }
            _ => None,
        }
    }

    fn try_keyword_with_block(
        t: &TokenTree,
        t2: Option<&TokenTree>,
        keyword: &str,
    ) -> Option<TokenStream> {
        match t {
            TokenTree::Ident(id) => {
                if *id == keyword {
                    if let Some(block) = t2 {
                        match syn::parse2::<syn::Block>(block.to_token_stream()) {
                            Err(e) => {
                                e.into_compile_error();
                            }
                            Ok(_) => return Some(block.to_token_stream()),
                        }
                    } else {
                        abort!(id, format!("expected code block after `{}`", keyword));
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn try_then(t: &TokenTree, t2: Option<&TokenTree>) -> Option<TokenStream> {
        try_keyword_with_block(t, t2, "then")
    }

    fn try_else(t: &TokenTree, t2: Option<&TokenTree>) -> Option<TokenStream> {
        try_keyword_with_block(t, t2, "else")
    }

    let mut args = None;
    let mut then_block = None;
    let mut else_block = None;
    if let Some(t) = tokens.next() {
        // 2rd tokentree found, check if it is args
        args = try_args(&t);
        if args.is_none() {
            // 2nd tokentree was not args, so tokentrees 2+3 must be a then+block or else+block
            let t2 = tokens.next();
            then_block = try_then(&t, t2.as_ref());
            else_block = try_else(&t, t2.as_ref());
            if then_block.is_none() && else_block.is_none() {
                abort!(
                    t,
                    "expected `(<args>)` or `then` or `else` followed by a code block"
                );
            }
        } else if let Some(t) = tokens.next() {
            // 3rd tokentree found, so tokentrees 3+4 must be then+block or else+block
            let t2 = tokens.next();
            then_block = try_then(&t, t2.as_ref());
            else_block = try_else(&t, t2.as_ref());
            if then_block.is_none() && else_block.is_none() {
                abort!(
                    t,
                    "expected `(<args>)` or `then` or `else` followed by a code block"
                );
            }
        }

        // if there are remaining tokentrees, there must be exactly 2 and they must be a else+block
        if let Some(t) = tokens.next() {
            let t2 = tokens.next();
            else_block = try_else(&t, t2.as_ref());
            if else_block.is_none() {
                abort!(t, "expected `else` followed by a code block");
            }
        }
    }

    let fun = Ident::new(&format!("parse_{}", id), id.span());

    return Parts {
        id,
        args,
        then_block,
        else_block,
        fun,
    };

    // return quote!(
    //     if let Some(#) = self.parse_unknown_at_rule() {
    //         self.append(node, unknown_at_rule);
    //     }
    // ).into()
}

#[proc_macro_error]
#[proc_macro]
pub fn addchild(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // set_dummy(quote!("Vec::<Vec<BlockType>>::new()"));
    let parts = addchild_internal(input);
    let id = parts.id;
    let args = parts.args;
    let then_block = parts.then_block;
    let else_block = parts.else_block;
    let fun = parts.fun;
    return quote!(
        if let Some(#id) = self.#fun(#args) {
            self.append(node, #id);
            #then_block
        } else {
            #else_block
        }
    )
    .into();
}

#[proc_macro_error]
#[proc_macro]
pub fn addchildbool(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // set_dummy(quote!("Vec::<Vec<BlockType>>::new()"));
    let parts = addchild_internal(input);
    let id = parts.id;
    let args = parts.args;
    let then_block = parts.then_block;
    let else_block = parts.else_block;
    let fun = parts.fun;
    return quote!(
        if let Some(#id) = self.#fun(#args) {
            self.append(node, #id);
            #then_block
            true
        } else {
            #else_block
            false
        }
    )
    .into();
}

#[proc_macro_error]
#[proc_macro]
pub fn assert_parse_node(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let tokens = proc_macro2::TokenStream::from(input);

    let mut i = tokens.into_iter();

    let tt = i
        .next()
        .unwrap_or_else(|| abort_call_site!("must pass text and ident for parsing func"));

    let TokenTree::Literal(text) = tt else {
        abort!(tt, "first value must be string literal");
    };

    let tt = i
        .next()
        .unwrap_or_else(|| abort_call_site!("must pass text and ident seperated by comma"));
    let TokenTree::Punct(_comma) = tt else {
        abort!(tt, "expected comma");
    };

    let tt = i
        .next()
        .unwrap_or_else(|| abort_call_site!("must pass text and ident for parsing func"));
    let TokenTree::Ident(id) = tt else {
        abort!(tt, "expected ident");
    };

    let args = if let Some(tt) = i.next() {
        // optional args for parse func
        if let TokenTree::Group(g) = tt {
            g.stream()
        } else {
            abort!(
                tt,
                "when including arguments for parse func, must be parenthesis delimited group"
            )
        }
    } else {
        quote!()
    };

    let fun = Ident::new(&format!("parse_{}", id), id.span());
    return quote!(assert_node(#text, |parser: &mut Parser| parser.#fun(#args))).into();
}

#[proc_macro_error]
#[proc_macro]
pub fn assert_parse_error(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let tokens = proc_macro2::TokenStream::from(input);

    let mut i = tokens.into_iter();

    let tt = i
        .next()
        .unwrap_or_else(|| abort_call_site!("must pass text, parse ident, and error ident"));

    let TokenTree::Literal(text) = tt else {
        abort!(tt, "first value must be string literal");
    };

    let tt = i
        .next()
        .unwrap_or_else(|| abort_call_site!("must pass text and ident seperated by comma"));
    let TokenTree::Punct(_comma) = tt else {
        abort!(tt, "expected comma");
    };

    let tt = i
        .next()
        .unwrap_or_else(|| abort_call_site!("must pass text and ident for parsing func"));
    let TokenTree::Ident(parse_ident) = tt else {
        abort!(tt, "expected parse func ident");
    };

    let tt = i.next().unwrap_or_else(|| {
        abort_call_site!("must pass text and ident for parsing func and error ident")
    });
    let (args, _comma_2) = if let TokenTree::Group(g) = tt {
        // optional args for parse func
        let tt = i.next().unwrap_or_else(|| {
            abort_call_site!("must pass comma-seperated text, parsing func ident, error ident")
        });
        if let TokenTree::Punct(comma_2) = tt {
            (g.stream(), comma_2)
        } else {
            abort!(tt, "");
        }
    } else if let TokenTree::Punct(comma_2) = tt {
        (quote!(), comma_2)
    } else {
        abort!(tt, "");
    };

    let tt = i
        .next()
        .unwrap_or_else(|| abort_call_site!("expected 3rd argument for error ident"));
    let TokenTree::Ident(error_ident) = tt else {
        abort!(tt, "expected error ident");
    };
    let parse_ident = Ident::new(&format!("parse_{}", parse_ident), parse_ident.span());
    return quote!(assert_error(#text, |parser: &mut Parser| parser.#parse_ident(#args), ParseError::#error_ident)).into();
}
