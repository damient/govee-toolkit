//! The Rust surface and the CLI surface, parsed from source.

use std::fs;
use std::path::{Path, PathBuf};

use quote::ToTokens;
use syn::{Attribute, Fields, FnArg, ImplItem, ImplItemFn, Item, Type, Visibility};

const HANDLES: [&str; 4] = ["Govee", "DeviceHandle", "Devices", "SegmentStream"];

/// `Type::name(arg: Type, …)`.
pub(super) fn methods(src: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for path in rust_files(src) {
        each_method(&parse(&path).items, &mut |owner, f| {
            let args: Vec<String> = f
                .sig
                .inputs
                .iter()
                .filter_map(|arg| match arg {
                    FnArg::Typed(t) => Some(format!(
                        "{}: {}",
                        tidy(&t.pat.to_token_stream().to_string()),
                        tidy(&t.ty.to_token_stream().to_string())
                    )),
                    FnArg::Receiver(_) => None,
                })
                .collect();
            out.push(format!("{owner}::{}({})", f.sig.ident, args.join(", ")));
        });
    }
    out.sort();
    out.dedup();
    out
}

/// `(role variant, Type::name)`.
pub(super) fn serves(src: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for path in rust_files(src) {
        each_method(&parse(&path).items, &mut |owner, f| {
            for role in doc_roles(&f.attrs) {
                out.push((role, format!("{owner}::{}", f.sig.ident)));
            }
        });
    }
    out.sort();
    out.dedup();
    out
}

fn doc_roles(attrs: &[Attribute]) -> Vec<String> {
    let doc: String = attrs
        .iter()
        .filter_map(|attr| match &attr.meta {
            syn::Meta::NameValue(pair) if pair.path.is_ident("doc") => match &pair.value {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) => Some(s.value() + " "),
                _ => None,
            },
            _ => None,
        })
        .collect();
    doc.split("[`Role::")
        .skip(1)
        .filter_map(|rest| rest.split_once("`]").map(|(role, _)| role.to_owned()))
        .collect()
}

fn each_method(items: &[Item], visit: &mut impl FnMut(&str, &ImplItemFn)) {
    for item in items {
        match item {
            Item::Mod(module) if !is_test(&module.attrs) => {
                if let Some((_, inner)) = &module.content {
                    each_method(inner, visit);
                }
            }
            Item::Impl(block) if block.trait_.is_none() => {
                let Some(owner) = last_ident(&block.self_ty) else {
                    continue;
                };
                if !HANDLES.contains(&owner.as_str()) {
                    continue;
                }
                for inner in &block.items {
                    if let ImplItem::Fn(f) = inner
                        && matches!(f.vis, Visibility::Public(_))
                    {
                        visit(&owner, f);
                    }
                }
            }
            _ => {}
        }
    }
}

/// `govee <kebab-name> <ARGS>`.
pub(super) fn cli(dir: &Path) -> Vec<String> {
    let items: Vec<Item> = rust_files(dir)
        .iter()
        .flat_map(|p| parse(p).items)
        .collect();
    let mut out = Vec::new();
    subcommands(&items, "Command", &mut out);
    out.sort();
    out
}

fn subcommands(items: &[Item], name: &str, out: &mut Vec<String>) {
    let Some(found) = items.iter().find_map(|item| match item {
        Item::Enum(e) if e.ident == name => Some(e),
        _ => None,
    }) else {
        panic!("the CLI declares no `enum {name}`");
    };
    for variant in &found.variants {
        if has_word(&variant.attrs, "command", "flatten") {
            let Fields::Unnamed(fields) = &variant.fields else {
                continue;
            };
            for field in &fields.unnamed {
                if let Some(inner) = last_ident(&field.ty) {
                    subcommands(items, &inner, out);
                }
            }
            continue;
        }
        let mut words = vec!["govee".to_owned(), kebab(&variant.ident.to_string())];
        if let Fields::Named(fields) = &variant.fields {
            words.extend(fields.named.iter().map(|field| {
                let ident = field
                    .ident
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default();
                argument(&ident, &field.ty, &field.attrs)
            }));
        }
        out.push(words.join(" "));
    }
}

/// One field, as clap renders it in a usage line.
fn argument(ident: &str, ty: &Type, attrs: &[Attribute]) -> String {
    let mut long = None;
    let mut value_name = None;
    for attr in attrs.iter().filter(|a| a.path().is_ident("arg")) {
        attr.parse_nested_meta(|meta| {
            let key = meta
                .path
                .get_ident()
                .map(ToString::to_string)
                .unwrap_or_default();
            let value = if meta.input.peek(syn::Token![=]) {
                Some(meta.value()?.parse::<syn::Expr>()?)
            } else {
                None
            };
            let literal = value.as_ref().and_then(|v| match v {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) => Some(s.value()),
                _ => None,
            });
            match key.as_str() {
                "long" => long = Some(literal.unwrap_or_else(|| ident.replace('_', "-"))),
                "value_name" => value_name = literal,
                _ => {}
            }
            Ok(())
        })
        .unwrap_or_else(|e| panic!("`#[arg]` on `{ident}`: {e}"));
    }
    let wrapper = last_ident(ty).unwrap_or_default();
    let value = value_name.unwrap_or_else(|| ident.to_uppercase());
    match (long, wrapper.as_str()) {
        (Some(flag), "bool") => format!("[--{flag}]"),
        (Some(flag), "Vec") => format!("[--{flag} <{value}>]..."),
        (Some(flag), _) => format!("[--{flag} <{value}>]"),
        (None, "Option") => format!("[{value}]"),
        (None, "Vec") => format!("[{value}]..."),
        (None, _) => format!("<{value}>"),
    }
}

fn has_word(attrs: &[Attribute], path: &str, word: &str) -> bool {
    attrs
        .iter()
        .filter(|a| a.path().is_ident(path))
        .any(|a| a.meta.to_token_stream().to_string().contains(word))
}

fn is_test(attrs: &[Attribute]) -> bool {
    has_word(attrs, "cfg", "test")
}

/// `Option<T>` and `Vec<T>` give the wrapper, not `T`.
pub(super) fn last_ident(ty: &Type) -> Option<String> {
    let Type::Path(path) = ty else { return None };
    Some(path.path.segments.last()?.ident.to_string())
}

pub(super) fn kebab(camel: &str) -> String {
    let mut out = String::new();
    for (i, c) in camel.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('-');
        }
        out.extend(c.to_lowercase());
    }
    out
}

/// A token stream as a person writes it: `&Paint<'_>`, not `& Paint < '_ >`.
fn tidy(tokens: &str) -> String {
    [
        (" :: ", "::"),
        (":: ", "::"),
        ("& ", "&"),
        (" <", "<"),
        ("< ", "<"),
        (" >", ">"),
        (" ,", ","),
        ("[ ", "["),
        (" ]", "]"),
        (" ;", ";"),
        ("( ", "("),
        (" )", ")"),
    ]
    .iter()
    .fold(tokens.to_owned(), |text, (from, to)| text.replace(from, to))
}

pub(super) fn parse(path: &Path) -> syn::File {
    syn::parse_file(&crate::read(path)).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in fs::read_dir(&next).unwrap_or_else(|e| panic!("{}: {e}", next.display())) {
            let path = entry.expect("read a directory entry").path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if name == "tests" || name == "tests.rs" {
                continue;
            }
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}
