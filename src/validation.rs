use crate::visitor::MatchFinder;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Arm, Expr, ExprMatch, ExprPath, ItemFn, Pat, Path, spanned::Spanned, visit::Visit};

/// Find the match inside `func` and validate its arms, panicking with a
/// user-facing message on any structural violation.
pub(crate) fn find_and_validate<'f>(attr: &str, func: &'f ItemFn) -> &'f [Arm] {
    let mut finder = MatchFinder { found: None };
    finder.visit_item_fn(func);

    let ExprMatch { arms, .. } = finder.found.unwrap_or_else(|| {
        panic!("{attr} can only be used on functions containing a match expression")
    });

    validate_enum_to_enum_arms(arms);
    arms
}

/// Each arm must map an enum variant path to an enum variant path.
/// We can only verify this syntactically: both the pattern and the body
/// must be plain paths (e.g. `Enum::Variant`), not literals, wildcards,
/// guards, tuple structs, or arbitrary expressions.
fn validate_enum_to_enum_arms(arms: &[Arm]) {
    assert!(
        !arms.is_empty(),
        "surject: match must have at least one arm"
    );

    for arm in arms {
        if arm.guard.is_some() {
            panic!("surject: match guards are not supported");
        }

        match &arm.pat {
            Pat::Path(_) => {}
            _ => panic!(
                "surject: every arm pattern must be an enum variant path (e.g. `Enum::Variant`)"
            ),
        }

        match arm.body.as_ref() {
            Expr::Path(_) => {}
            _ => panic!(
                "surject: every arm body must be an enum variant path (e.g. `Enum::Variant`)"
            ),
        }
    }
}

/// Returns a `compile_error!` token stream pointing at the first duplicate
/// output path, or `None` if the mapping is injective.
pub(crate) fn check_injectivity(arms: &[Arm]) -> Option<TokenStream2> {
    let mut seen: Vec<(String, proc_macro2::Span)> = Vec::new();

    for arm in arms {
        let Expr::Path(output) = arm.body.as_ref() else {
            unreachable!("already validated")
        };
        let key = quote!(#output).to_string();

        if seen.iter().any(|(k, _)| k == &key) {
            return Some(
                syn::Error::new(
                    output.span(),
                    format!(
                        "inject: `{key}` is produced by more than one arm; \
                         the mapping is not injective"
                    ),
                )
                .to_compile_error(),
            );
        }

        seen.push((key, output.span()));
    }

    None
}

/// Build the arms for the compiler-checked surjectivity function.
/// Each unique output variant seen across all arms produces one arm mapping to `()`.
/// If any variant of the output enum is absent the compiler will report a
/// non-exhaustive match, which is exactly the surjectivity check we want.
pub(crate) fn surjectivity_check_arms(arms: &[Arm]) -> Vec<TokenStream2> {
    let mut seen: Vec<String> = Vec::new();
    let mut unique_outputs: Vec<ExprPath> = Vec::new();

    for arm in arms {
        let Expr::Path(output) = arm.body.as_ref() else {
            unreachable!("already validated")
        };
        let key = quote!(#output).to_string();
        if !seen.contains(&key) {
            seen.push(key);
            unique_outputs.push(output.clone());
        }
    }

    unique_outputs
        .iter()
        .map(|output| quote! { #output => (), })
        .collect()
}

/// Returns all but the last path segment — the enum type without the variant.
/// e.g. `Letters::A` -> `Letters`
///
/// We rebuild the path from scratch rather than using `Punctuated::pop`, which
/// leaves a dangling trailing `::` in the punctuated sequence.
fn enum_type_of_path(path: &Path) -> Path {
    let n = path.segments.len();
    assert!(
        n >= 2,
        "surject: enum path must have at least 2 segments (e.g. `Enum::Variant`), got: `{}`",
        quote::quote!(#path),
    );

    let mut segments = syn::punctuated::Punctuated::new();
    for seg in path.segments.iter().take(n - 1) {
        segments.push(seg.clone());
    }

    Path {
        leading_colon: path.leading_colon,
        segments,
    }
}

pub(crate) fn enum_type_of_expr(expr: &ExprPath) -> Path {
    enum_type_of_path(&expr.path)
}

pub(crate) trait AsExprPath {
    fn as_expr_path(&self) -> &ExprPath;
}

impl AsExprPath for Expr {
    fn as_expr_path(&self) -> &ExprPath {
        let Expr::Path(p) = self else {
            panic!("expected Expr::Path")
        };
        p
    }
}
