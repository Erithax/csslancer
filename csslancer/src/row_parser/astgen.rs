// truly one of the rust files of all time

use super::syntax_kind_src::{SYNTAX_KINDS_SRC, SyntaxKindsSrc};
use quote::{format_ident, quote};
use proc_macro2::{Punct, Spacing};
use xshell::{cmd, Shell};

pub fn generate(check: bool) {
    //csslancer::parser2::grammar_generator::sourcegen_ast()
    let syntax_kinds = generate_syntax_kinds(SYNTAX_KINDS_SRC);
    let syntax_kinds_file = project_root().join("csslancer/src/row_parser/syntax_kind_gen.rs");
    ensure_file_contents(syntax_kinds_file.as_path(), &syntax_kinds, check);
}

/// Returns the path to the root directory of `csslancer` project.
fn project_root() -> std::path::PathBuf {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_owned());
    std::path::PathBuf::from(dir).parent().unwrap().to_owned()
}

/// Checks that the `file` has the specified `contents`. If that is not the
/// case, updates the file and then fails the test.
#[allow(clippy::print_stderr)]
fn ensure_file_contents(file: &std::path::Path, contents: &str, check: bool) {
    if let Ok(old_contents) = std::fs::read_to_string(file) {
        if normalize_newlines(&old_contents) == normalize_newlines(contents) {
            // File is already up to date.
            return;
        }
    }

    let display_path = file.strip_prefix(project_root()).unwrap_or(file);
    if check {
        panic!(
            "{} was not up-to-date{}",
            file.display(),
            if std::env::var("CI").is_ok() {
                "\n    NOTE: run `cargo codegen` locally and commit the updated files\n"
            } else {
                ""
            }
        );
    } else {
        eprintln!(
            "\n\x1b[31;1merror\x1b[0m: {} was not up-to-date, updating\n",
            display_path.display()
        );

        if let Some(parent) = file.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(file, contents).unwrap();
    }
}

fn normalize_newlines(s: &str) -> String {
    s.replace("\r\n", "\n")
}

fn generate_syntax_kinds(grammar: SyntaxKindsSrc<'_>) -> String {
    let (single_byte_tokens_values, single_byte_tokens): (Vec<_>, Vec<_>) = grammar
        .punct
        .iter()
        .filter(|(token, _name)| token.len() == 1)
        .map(|(token, name)| (token.chars().next().unwrap(), format_ident!("{}", name)))
        .unzip();

    let punctuation_values = grammar.punct.iter().map(|(token, _name)| {
        if "{}[]()".contains(token) {
            let c = token.chars().next().unwrap();
            quote! { #c }
        } else {
            let cs = token.chars().map(|c| Punct::new(c, Spacing::Joint));
            quote! { #(#cs)* }
        }
    });
    let punctuation =
        grammar.punct.iter().map(|(_token, name)| format_ident!("{}", name)).collect::<Vec<_>>();

    let dim_str_to_id = |name: &str| match name {
        "%" => format_ident!("DIM_PERCENT"),
        name => format_ident!("DIM_{}", to_upper_snake_case(name)),
    };

    let dimensions = grammar.dimensions.split_whitespace().map(|s| dim_str_to_id(s)).collect::<Vec<_>>();

    let tokens = grammar.tokens.iter().map(|name| format_ident!("{}", name)).collect::<Vec<_>>();

    let css_nodes = grammar.css_nodes.iter().map(|n| format_ident!("{n}")).collect::<Vec<_>>();
    let xcss_nodes = grammar.xcss_nodes.iter().map(|n| format_ident!("XCSS_{n}")).collect::<Vec<_>>();
    let scss_nodes = grammar.scss_nodes.iter().map(|n| format_ident!("SCSS_{n}")).collect::<Vec<_>>();
    let less_nodes = grammar.less_nodes.iter().map(|n| format_ident!("LESS_{n}")).collect::<Vec<_>>();


    let ast = quote! {
        #![allow(bad_style, missing_docs, unreachable_pub)]
        /// The kind of syntax node, e.g. `IDENT`, `USE_KW`, or `STRUCT`.
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        #[repr(u16)]
        pub enum SyntaxKind {
            // Technical SyntaxKinds: they appear temporally during parsing,
            // but never end up in the final tree
            #[doc(hidden)]
            TOMBSTONE,
            #[doc(hidden)]
            EOF,
            #(#tokens,)*
            #(#punctuation,)*
            #(#dimensions,)*
            #(#css_nodes,)*
            #(#xcss_nodes,)*
            #(#scss_nodes,)*
            #(#less_nodes,)*

            // Technical kind so that we can cast from u16 safely
            #[doc(hidden)]
            __LAST,
        }
        use self::SyntaxKind::*;

        impl SyntaxKind {
            pub fn is_dimension(self) -> bool {
                matches!(self, #(#dimensions)|*)
            }

            pub fn is_punct(self) -> bool {
                matches!(self, #(#punctuation)|*)
            }

            // pub fn from_keyword(ident: &str) -> Option<SyntaxKind> {
            //     let kw = match ident {
            //         #(#full_keywords_values => #full_keywords,)*
            //         _ => return None,
            //     };
            //     Some(kw)
            // }

            // pub fn from_contextual_keyword(ident: &str) -> Option<SyntaxKind> {
            //     let kw = match ident {
            //         #(#contextual_keywords_values => #contextual_keywords,)*
            //         _ => return None,
            //     };
            //     Some(kw)
            // }

            pub fn from_char(c: char) -> Option<SyntaxKind> {
                let tok = match c {
                    #(#single_byte_tokens_values => #single_byte_tokens,)*
                    _ => return None,
                };
                Some(tok)
            }
        }

        #[macro_export]
        macro_rules! T {
            #([#punctuation_values] => { $crate::SyntaxKind::#punctuation };)*
            #([#dimensions] => { $crate::SyntaxKind::#dimensions };)*
            [ident] => { $crate::SyntaxKind::IDENT };
        }
    };

    add_preamble("sourcegen_ast", reformat(ast.to_string()))
}

fn add_preamble(generator: &'static str, mut text: String) -> String {
    let preamble = format!("//! Generated by `{generator}`, do not edit by hand.\n\n");
    text.insert_str(0, &preamble);
    text
}

fn ensure_rustfmt(sh: &Shell) {
    let version = cmd!(sh, "rustup run stable rustfmt --version").read().unwrap_or_default();
    if !version.contains("stable") {
        panic!(
            "Failed to run rustfmt from toolchain 'stable'. \
                 Please run `rustup component add rustfmt --toolchain stable` to install it.",
        );
    }
}


fn reformat(text: String) -> String {
    let sh = Shell::new().unwrap();
    ensure_rustfmt(&sh);
    let rustfmt_toml = project_root().join("rustfmt.toml");
    println!("{text}");
    let rustfmt_toml_arg = if rustfmt_toml.exists() {
        println!("RUST TOML: {}", rustfmt_toml.as_os_str().to_str().unwrap());
        format!("--config-path {} ", rustfmt_toml.as_os_str().to_str().unwrap())
    } else {
        String::new()
    };
    let mut stdout = cmd!(
        sh,
        "rustup run stable rustfmt {rustfmt_toml_arg}--config fn_single_line=true"
    )
    .stdin(text)
    .read()
    .unwrap();
    if !stdout.ends_with('\n') {
        stdout.push('\n');
    }
    stdout
}

fn to_upper_snake_case(s: &str) -> String {
    let mut buf = String::with_capacity(s.len());
    let mut prev = false;
    for c in s.chars() {
        if c.is_ascii_uppercase() && prev {
            buf.push('_')
        }
        prev = true;

        buf.push(c.to_ascii_uppercase());
    }
    buf
}