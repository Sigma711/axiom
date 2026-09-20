//! Shared build/test AST indexer: no source line numbers are maintained by hand.
use proc_macro2::Span;
use quote::ToTokens;
use std::collections::BTreeMap;
use syn::{
    spanned::Spanned,
    visit::{self, Visit},
};
#[derive(Debug, Clone)]
pub struct SourceSymbol {
    pub path: String,
    pub symbol: String,
    pub line: usize,
    pub end_line: usize,
    pub kind: &'static str,
}
pub fn scan(path: &str, source: &str) -> Result<Vec<SourceSymbol>, syn::Error> {
    let file = syn::parse_file(source)?;
    let mut index = Indexer {
        path: path.into(),
        scope: vec![],
        function: None,
        rows: BTreeMap::new(),
    };
    index.visit_file(&file);
    Ok(index.rows.into_values().collect())
}
struct Indexer {
    path: String,
    scope: Vec<String>,
    function: Option<String>,
    rows: BTreeMap<String, SourceSymbol>,
}
impl Indexer {
    fn qualified(&self, name: &str) -> String {
        self.scope
            .iter()
            .cloned()
            .chain(std::iter::once(name.into()))
            .collect::<Vec<_>>()
            .join("::")
    }
    fn insert(&mut self, symbol: String, start: Span, end: Span, kind: &'static str) {
        let row = SourceSymbol {
            path: self.path.clone(),
            symbol: symbol.clone(),
            line: start.start().line,
            end_line: end.end().line,
            kind,
        };
        // Prefer the innermost implementation when an outer grouped arm dispatches again.
        let replace = self.rows.get(&symbol).is_none_or(|old| {
            row.end_line.saturating_sub(row.line) <= old.end_line.saturating_sub(old.line)
        });
        if replace {
            self.rows.insert(symbol, row);
        }
    }
    fn cases(&mut self, pat: &syn::Pat, start: Span, end: Span) {
        let Some(function) = self.function.clone() else {
            return;
        };
        for value in pattern_strings(pat) {
            self.insert(format!("{function}::{value}"), start, end, "match_arm");
        }
    }
}
fn pattern_strings(pat: &syn::Pat) -> Vec<String> {
    match pat {
        syn::Pat::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        }) => vec![s.value()],
        syn::Pat::Or(p) => p.cases.iter().flat_map(pattern_strings).collect(),
        syn::Pat::Paren(p) => pattern_strings(&p.pat),
        syn::Pat::Reference(p) => pattern_strings(&p.pat),
        _ => vec![],
    }
}
impl<'ast> Visit<'ast> for Indexer {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        let name = self.qualified(&item.sig.ident.to_string());
        self.insert(name.clone(), item.sig.span(), item.block.span(), "function");
        let old = self.function.replace(name);
        visit::visit_item_fn(self, item);
        self.function = old;
    }
    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        let name = self.qualified(&item.sig.ident.to_string());
        self.insert(name.clone(), item.sig.span(), item.block.span(), "method");
        let old = self.function.replace(name);
        visit::visit_impl_item_fn(self, item);
        self.function = old;
    }
    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        let name = item.self_ty.to_token_stream().to_string().replace(' ', "");
        self.scope.push(name);
        visit::visit_item_impl(self, item);
        self.scope.pop();
    }
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        self.scope.push(item.ident.to_string());
        visit::visit_item_mod(self, item);
        self.scope.pop();
    }
    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        self.insert(
            self.qualified(&item.ident.to_string()),
            item.struct_token.span,
            item.span(),
            "struct",
        );
        visit::visit_item_struct(self, item);
    }
    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        self.insert(
            self.qualified(&item.ident.to_string()),
            item.enum_token.span,
            item.span(),
            "enum",
        );
        visit::visit_item_enum(self, item);
    }
    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.insert(
            self.qualified(&item.ident.to_string()),
            item.trait_token.span,
            item.span(),
            "trait",
        );
        visit::visit_item_trait(self, item);
    }
    fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
        self.insert(
            self.qualified(&item.ident.to_string()),
            item.type_token.span,
            item.span(),
            "type",
        );
        visit::visit_item_type(self, item);
    }
    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        self.cases(&arm.pat, arm.pat.span(), arm.body.span());
        visit::visit_arm(self, arm);
    }
    fn visit_expr_if(&mut self, item: &'ast syn::ExprIf) {
        // Older grouped evaluators use literal equality/contains conditions. These
        // references point to the actual selected conditional, not a metadata row.
        if let Some(function) = self.function.clone() {
            let mut strings = StringLiterals(Vec::new());
            strings.visit_expr(&item.cond);
            for value in strings.0 {
                self.insert(
                    format!("{function}::{value}"),
                    item.cond.span(),
                    item.then_branch.span(),
                    "conditional",
                );
            }
        }
        visit::visit_expr_if(self, item);
    }
}
struct StringLiterals(Vec<String>);
impl<'ast> Visit<'ast> for StringLiterals {
    fn visit_lit_str(&mut self, v: &'ast syn::LitStr) {
        self.0.push(v.value());
    }
}
