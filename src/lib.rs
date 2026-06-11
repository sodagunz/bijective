use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Arm, Expr, ExprMatch, ExprPath, Pat, Path, visit::Visit};

struct MatchFinder<'ast> {
    found: Option<&'ast ExprMatch>,
}

impl<'ast> Visit<'ast> for MatchFinder<'ast> {
    fn visit_expr_match(&mut self, node: &'ast ExprMatch) {
        if self.found.is_none() {
            self.found = Some(node);
            // Don't delegate to the default impl — stops recursion into nested
            // matches inside arms, so we always capture the outermost one.
        }
    }
}

#[proc_macro_attribute]
pub fn surject(attr: TokenStream, input: TokenStream) -> TokenStream {
    let ast: Expr = syn::parse(input).unwrap(); // FIXME
    let attr = attr.to_string();
    impl_surject_macro(&attr, &ast).into()
}

fn impl_surject_macro(attr: &str, ast: &Expr) -> TokenStream2 {
    let mut finder = MatchFinder { found: None };
    finder.visit_expr(ast);

    let ExprMatch {
        expr: scrutinee,
        arms,
        ..
    } = finder
        .found
        .unwrap_or_else(|| panic!("{attr} can only be used on (or around) a match expression"));

    validate_enum_to_enum_arms(arms);

    let output_type = enum_type_of_expr(arms[0].body.as_expr_path());
    let check_arms = surjectivity_check_arms(arms);

    quote! {
        {
            // This function is never called; it exists solely so the compiler
            // verifies that every variant of the output type appears at least
            // once as an arm body (i.e. the mapping is surjective).
            #[expect(dead_code)]
            fn surjectivity_check(output: #output_type) {
                match output {
                    #(#check_arms)*
                }
            }

            match #scrutinee {
                #(#arms)*
            }
        }
    }
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

fn enum_type_of_expr(expr: &ExprPath) -> Path {
    enum_type_of_path(&expr.path)
}

/// Build the arms for the compiler-checked surjectivity function.
/// Each unique output variant seen across all arms produces one arm mapping to `()`.
/// If any variant of the output enum is absent the compiler will report a
/// non-exhaustive match, which is exactly the surjectivity check we want.
fn surjectivity_check_arms(arms: &[Arm]) -> Vec<TokenStream2> {
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

// -- helpers for tests -------------------------------------------------------

trait AsExprPath {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(code: &str) -> Expr {
        syn::parse_str(code).expect("test input failed to parse")
    }

    fn parse_output(code: &str) -> Expr {
        let output = impl_surject_macro("surject", &parse(code));
        println!("{output}");
        syn::parse2(output).expect("macro output should be a valid expression")
    }

    // -- valid inputs ---------------------------------------------------------

    #[test]
    fn generates_block_with_forward_match() {
        let Expr::Block(block) =
            parse_output("match a { Letters::A => Letters::D, Letters::B => Letters::C }")
        else {
            panic!("expected block expression");
        };

        // Last statement is the forward match expression.
        let stmts = &block.block.stmts;
        assert_eq!(
            stmts.len(),
            2,
            "expected surjectivity_check fn + forward match"
        );

        let syn::Stmt::Expr(Expr::Match(forward), _) = stmts.last().unwrap() else {
            panic!("expected match as last statement");
        };
        assert_eq!(forward.arms.len(), 2);
    }

    #[test]
    fn surjectivity_check_deduplicates_outputs() {
        // Two inputs map to the same output — a genuine surjection.
        let Expr::Block(block) = parse_output(
            "match d {
                Dir::North => Axis::Vertical,
                Dir::South => Axis::Vertical,
                Dir::East  => Axis::Horizontal,
                Dir::West  => Axis::Horizontal,
            }",
        ) else {
            panic!("expected block expression");
        };

        // First statement is the `fn surjectivity_check(...)` item.
        let syn::Stmt::Item(syn::Item::Fn(check_fn)) = block.block.stmts.first().unwrap() else {
            panic!("expected fn item as first statement");
        };
        assert_eq!(check_fn.sig.ident, "surjectivity_check");

        // The check function body has one arm per *unique* output variant
        // (2: Vertical and Horizontal), not one per input arm (4).
        let syn::Stmt::Expr(Expr::Match(inner), _) = check_fn.block.stmts.first().unwrap() else {
            panic!("expected match inside surjectivity_check");
        };
        assert_eq!(inner.arms.len(), 2, "one arm per unique output variant");
    }

    #[test]
    fn match_inside_let_is_found() {
        let expr = parse(
            "let b = match a {
                Letters::A => Letters::D,
                Letters::B => Letters::C,
                Letters::C => Letters::B,
                Letters::D => Letters::A,
            }",
        );
        let output = impl_surject_macro("surject", &expr);
        assert!(!output.is_empty());
        println!("{output}");
    }

    // -- invalid: no match expression -----------------------------------------

    #[test]
    #[should_panic(expected = "can only be used on (or around) a match expression")]
    fn non_match_expr_panics() {
        impl_surject_macro("surject", &parse("a + b"));
    }

    // -- invalid: arm patterns are not enum variant paths ---------------------

    #[test]
    #[should_panic(expected = "arm pattern must be an enum variant path")]
    fn wildcard_pattern_panics() {
        impl_surject_macro("surject", &parse("match a { _ => Letters::A }"));
    }

    #[test]
    #[should_panic(expected = "arm pattern must be an enum variant path")]
    fn literal_pattern_panics() {
        impl_surject_macro("surject", &parse("match a { 1 => Letters::A }"));
    }

    // -- invalid: arm bodies are not enum variant paths -----------------------

    #[test]
    #[should_panic(expected = "arm body must be an enum variant path")]
    fn call_expression_body_panics() {
        impl_surject_macro(
            "surject",
            &parse("match a { Letters::A => some_fn(), Letters::B => Letters::C }"),
        );
    }

    #[test]
    #[should_panic(expected = "arm body must be an enum variant path")]
    fn literal_body_panics() {
        impl_surject_macro("surject", &parse("match a { Letters::A => 42 }"));
    }

    // -- invalid: guards are not allowed --------------------------------------

    #[test]
    #[should_panic(expected = "match guards are not supported")]
    fn guard_panics() {
        impl_surject_macro(
            "surject",
            &parse("match a { Letters::A if cond => Letters::B }"),
        );
    }
}
