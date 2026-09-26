//! The CLI verbs that reach each facade method, read from the dispatch in
//! `crates/cli/src/run/mod.rs`.

use std::collections::BTreeMap;
use std::path::Path;

use syn::visit::{self, Visit};
use syn::{Expr, ExprCall, ExprMatch, ExprMethodCall, ExprPath, ExprStruct, Item, ItemFn, Pat};

use super::rust::{kebab, parse};

/// Method name to the CLI verbs that call it, sorted.
///
/// An arm of `route` reaches every method that the `<module>::<fn>` it calls
/// calls in `run/<module>.rs`. An arm of `play` reaches the method that its
/// `Verb` variant names: `Verb::ColorTemp` reaches `color_temp`.
pub(super) fn verbs(run: &Path) -> BTreeMap<String, Vec<String>> {
    let items = parse(&run.join("mod.rs")).items;
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (word, body) in arms(&items, "route") {
        for (module, name) in Calls::of(body).paths {
            let path = run.join(format!("{module}.rs"));
            if !path.exists() {
                continue;
            }
            let file = parse(&path).items;
            if let Some(f) = function(&file, &name) {
                for method in Calls::of_fn(f).methods {
                    out.entry(method).or_default().push(word.clone());
                }
            }
        }
    }
    for (word, body) in arms(&items, "play") {
        let calls = Calls::of(body);
        let helpers = calls
            .functions
            .iter()
            .filter_map(|name| function(&items, name));
        let variants = calls
            .verbs
            .into_iter()
            .chain(helpers.flat_map(|f| Calls::of_fn(f).verbs));
        for variant in variants {
            out.entry(snake(&variant)).or_default().push(word.clone());
        }
    }
    for verbs in out.values_mut() {
        verbs.sort();
        verbs.dedup();
    }
    out
}

/// The verb and the body of each arm of the first `match` in `name`.
fn arms<'a>(items: &'a [Item], name: &str) -> Vec<(String, &'a Expr)> {
    let f = function(items, name).unwrap_or_else(|| panic!("the CLI declares no `fn {name}`"));
    let mut first = FirstMatch(None);
    first.visit_item_fn(f);
    let Some(found) = first.0 else {
        panic!("`fn {name}` in the CLI holds no `match`");
    };
    found
        .arms
        .iter()
        .filter_map(|arm| Some((kebab(&variant(&arm.pat)?), arm.body.as_ref())))
        .collect()
}

/// The last segment of the path an arm matches: `Status` for
/// `Command::Status { .. }`.
fn variant(pat: &Pat) -> Option<String> {
    let path = match pat {
        Pat::Path(p) => &p.path,
        Pat::Struct(p) => &p.path,
        Pat::TupleStruct(p) => &p.path,
        _ => return None,
    };
    Some(path.segments.last()?.ident.to_string())
}

fn function<'a>(items: &'a [Item], name: &str) -> Option<&'a ItemFn> {
    items.iter().find_map(|item| match item {
        Item::Fn(f) if f.sig.ident == name => Some(f),
        _ => None,
    })
}

struct FirstMatch<'a>(Option<&'a ExprMatch>);

impl<'a> Visit<'a> for FirstMatch<'a> {
    fn visit_expr_match(&mut self, found: &'a ExprMatch) {
        if self.0.is_none() {
            self.0 = Some(found);
        }
    }
}

/// What one expression calls and names.
#[derive(Default)]
struct Calls {
    /// `module::name(…)`.
    paths: Vec<(String, String)>,
    /// `name(…)`.
    functions: Vec<String>,
    /// `x.name(…)`.
    methods: Vec<String>,
    /// The variant of every `Verb::Variant`.
    verbs: Vec<String>,
}

impl Calls {
    fn of(expr: &Expr) -> Self {
        let mut calls = Self::default();
        calls.visit_expr(expr);
        calls
    }

    fn of_fn(f: &ItemFn) -> Self {
        let mut calls = Self::default();
        calls.visit_item_fn(f);
        calls
    }

    fn verb(&mut self, path: &syn::Path) {
        if let [ty, variant] = path.segments.iter().collect::<Vec<_>>()[..]
            && ty.ident == "Verb"
        {
            self.verbs.push(variant.ident.to_string());
        }
    }
}

impl<'a> Visit<'a> for Calls {
    fn visit_expr_call(&mut self, call: &'a ExprCall) {
        if let Expr::Path(ExprPath { path, .. }) = call.func.as_ref() {
            let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
            match &segments[..] {
                [name] => self.functions.push(name.clone()),
                [module, name] => self.paths.push((module.clone(), name.clone())),
                _ => {}
            }
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'a ExprMethodCall) {
        self.methods.push(call.method.to_string());
        visit::visit_expr_method_call(self, call);
    }

    fn visit_expr_path(&mut self, path: &'a ExprPath) {
        self.verb(&path.path);
        visit::visit_expr_path(self, path);
    }

    fn visit_expr_struct(&mut self, found: &'a ExprStruct) {
        self.verb(&found.path);
        visit::visit_expr_struct(self, found);
    }
}

/// `ColorTemp` to `color_temp`.
fn snake(camel: &str) -> String {
    kebab(camel).replace('-', "_")
}
