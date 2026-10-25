//! Computing the subset of an entity's generics each generated type needs.

use std::collections::HashSet;

use syn::punctuated::Punctuated;
use syn::{GenericParam, Generics, WhereClause};

use crate::types::idents_in;

fn param_name(p: &GenericParam) -> String {
    match p {
        GenericParam::Type(t) => t.ident.to_string(),
        GenericParam::Lifetime(l) => format!("'{}", l.lifetime.ident),
        GenericParam::Const(c) => c.ident.to_string(),
    }
}

/// Names of all generic parameters (lifetimes are `'a`-keyed).
pub fn param_names(g: &Generics) -> HashSet<String> {
    g.params.iter().map(param_name).collect()
}

/// Generics restricted to the parameters mentioned in `used`, closed over the
/// bounds and where-predicates that tie parameters together. Defaults are
/// dropped; where-predicates are kept only if they mention nothing outside
/// the kept set.
pub fn subset(all: &Generics, used: &HashSet<String>) -> Generics {
    let names = param_names(all);
    let mut kept: HashSet<String> = names.intersection(used).cloned().collect();

    loop {
        let before = kept.len();
        for p in &all.params {
            if kept.contains(&param_name(p)) {
                kept.extend(names.intersection(&bounds_idents(p)).cloned());
            }
        }
        if let Some(wc) = &all.where_clause {
            for pred in &wc.predicates {
                let lhs = match pred {
                    syn::WherePredicate::Type(t) => idents_in(&t.bounded_ty),
                    syn::WherePredicate::Lifetime(l) => idents_in(&l.lifetime),
                    _ => continue,
                };
                if !names.intersection(&lhs).all(|n| !kept.contains(n)) {
                    kept.extend(names.intersection(&idents_in(pred)).cloned());
                }
            }
        }
        if kept.len() == before {
            break;
        }
    }

    let params: Punctuated<GenericParam, syn::Token![,]> = all
        .params
        .iter()
        .filter(|p| kept.contains(&param_name(p)))
        .map(strip_defaults)
        .collect();

    if params.is_empty() {
        return Generics::default();
    }

    let predicates: Punctuated<syn::WherePredicate, syn::Token![,]> = all
        .where_clause
        .iter()
        .flat_map(|wc| wc.predicates.iter())
        .filter(|pred| {
            let mentioned: HashSet<String> =
                names.intersection(&idents_in(*pred)).cloned().collect();
            !mentioned.is_empty() && mentioned.is_subset(&kept)
        })
        .cloned()
        .collect();

    Generics {
        lt_token: Some(Default::default()),
        params,
        gt_token: Some(Default::default()),
        where_clause: (!predicates.is_empty()).then_some(WhereClause {
            where_token: Default::default(),
            predicates,
        }),
    }
}

/// The parameter without its `= Default`.
fn strip_defaults(p: &GenericParam) -> GenericParam {
    let mut p = p.clone();
    match &mut p {
        GenericParam::Type(t) => {
            t.eq_token = None;
            t.default = None;
        }
        GenericParam::Const(c) => {
            c.eq_token = None;
            c.default = None;
        }
        GenericParam::Lifetime(_) => {}
    }
    p
}

fn bounds_idents(p: &GenericParam) -> HashSet<String> {
    match p {
        GenericParam::Type(t) => idents_in(&t.bounds),
        GenericParam::Lifetime(l) => idents_in(&l.bounds),
        GenericParam::Const(c) => idents_in(&c.ty),
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;
    use syn::parse_quote;

    use super::*;

    fn used(names: &[&str]) -> HashSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn keeps_only_used_params() {
        let g: Generics = parse_quote!(<T: Clone, U: Default>);
        let s = subset(&g, &used(&["U"]));
        assert_eq!(s.params.len(), 1);
        assert!(s.to_token_stream().to_string().contains('U'));
    }

    #[test]
    fn pulls_in_params_referenced_by_bounds() {
        let g: Generics = parse_quote!(<T, U: From<T>>);
        assert_eq!(subset(&g, &used(&["U"])).params.len(), 2);
    }

    #[test]
    fn drops_predicates_on_unused_params() {
        let item: syn::DeriveInput = parse_quote!(
            struct S<T, U>
            where
                T: Clone,
                U: Copy;
        );
        let g = item.generics;
        let s = subset(&g, &used(&["T"]));
        assert_eq!(s.where_clause.unwrap().predicates.len(), 1);
    }

    #[test]
    fn no_params_gives_empty_generics() {
        let g: Generics = parse_quote!(<T>);
        assert!(subset(&g, &used(&[])).params.is_empty());
    }
}
