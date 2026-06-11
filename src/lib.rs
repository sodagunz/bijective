use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Arm, Expr, ExprMatch, ExprPath, ItemFn, Pat, Path, spanned::Spanned, visit::Visit};

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
    let func: ItemFn = syn::parse(input).unwrap();
    let attr = attr.to_string();
    impl_surject_macro(&attr, &func).into()
}

/// Alias for `#[surject]`.
#[proc_macro_attribute]
pub fn onto(attr: TokenStream, input: TokenStream) -> TokenStream {
    surject(attr, input)
}

fn impl_surject_macro(attr: &str, func: &ItemFn) -> TokenStream2 {
    let mut finder = MatchFinder { found: None };
    finder.visit_item_fn(func);

    let ExprMatch { arms, .. } = finder.found.unwrap_or_else(|| {
        panic!("{attr} can only be used on functions containing a match expression")
    });

    validate_enum_to_enum_arms(arms);

    let output_type = enum_type_of_expr(arms[0].body.as_expr_path());
    let check_arms = surjectivity_check_arms(arms);
    let check_fn_name = format_ident!("surjectivity_check_{}", func.sig.ident);

    quote! {
        #func

        // Never called; exists solely so the compiler verifies that every
        // variant of the output type appears at least once as an arm body.
        #[expect(dead_code)]
        fn #check_fn_name(output: #output_type) {
            match output {
                #(#check_arms)*
            }
        }
    }
}

#[proc_macro_attribute]
pub fn inject(attr: TokenStream, input: TokenStream) -> TokenStream {
    let func: ItemFn = syn::parse(input).unwrap();
    let attr = attr.to_string();
    impl_inject_macro(&attr, &func).into()
}

/// Alias for `#[inject]`.
#[proc_macro_attribute]
pub fn one_to_one(attr: TokenStream, input: TokenStream) -> TokenStream {
    inject(attr, input)
}

#[proc_macro_attribute]
pub fn biject(attr: TokenStream, input: TokenStream) -> TokenStream {
    let func: ItemFn = syn::parse(input).unwrap();
    let attr = attr.to_string();
    impl_biject_macro(&attr, &func).into()
}

fn impl_biject_macro(attr: &str, func: &ItemFn) -> TokenStream2 {
    let mut finder = MatchFinder { found: None };
    finder.visit_item_fn(func);

    let ExprMatch { arms, .. } = finder.found.unwrap_or_else(|| {
        panic!("{attr} can only be used on functions containing a match expression")
    });

    validate_enum_to_enum_arms(arms);

    // Injectivity is checked at expansion time; bail out early with a
    // span-accurate error before generating the surjectivity check.
    if let Some(err) = check_injectivity(arms) {
        return err;
    }

    // Surjectivity is delegated to the compiler via exhaustiveness checking.
    let output_type = enum_type_of_expr(arms[0].body.as_expr_path());
    let check_arms = surjectivity_check_arms(arms);
    let check_fn_name = format_ident!("bijectivity_check_{}", func.sig.ident);

    quote! {
        #func

        #[expect(dead_code)]
        fn #check_fn_name(output: #output_type) {
            match output {
                #(#check_arms)*
            }
        }
    }
}

fn impl_inject_macro(attr: &str, func: &ItemFn) -> TokenStream2 {
    let mut finder = MatchFinder { found: None };
    finder.visit_item_fn(func);

    let ExprMatch { arms, .. } = finder.found.unwrap_or_else(|| {
        panic!("{attr} can only be used on functions containing a match expression")
    });

    validate_enum_to_enum_arms(arms);

    if let Some(err) = check_injectivity(arms) {
        return err;
    }

    quote! { #func }
}

/// Returns a `compile_error!` token stream pointing at the first duplicate
/// output path, or `None` if the mapping is injective.
fn check_injectivity(arms: &[Arm]) -> Option<TokenStream2> {
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
                    format!("inject: `{key}` is produced by more than one arm; the mapping is not injective"),
                )
                .to_compile_error(),
            );
        }

        seen.push((key, output.span()));
    }

    None
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

    fn run(code: &str) -> TokenStream2 {
        let func: ItemFn = syn::parse_str(code).expect("test input failed to parse");
        impl_surject_macro("surject", &func)
    }

    fn parse_items(code: &str) -> Vec<syn::Item> {
        let output = run(code);
        println!("{output}");
        syn::parse_file(&output.to_string())
            .expect("output should be valid items")
            .items
    }

    // -- valid inputs ---------------------------------------------------------

    #[test]
    fn generates_original_fn_and_check_fn() {
        let items = parse_items(
            "fn map(l: Letter) -> Letter {
                match l {
                    Letter::A => Letter::D,
                    Letter::B => Letter::C,
                    Letter::C => Letter::B,
                    Letter::D => Letter::A,
                }
            }",
        );

        assert_eq!(
            items.len(),
            2,
            "expected original fn + surjectivity_check fn"
        );

        let syn::Item::Fn(check_fn) = &items[1] else {
            panic!("second item should be a fn");
        };
        assert_eq!(check_fn.sig.ident, "surjectivity_check_map");
    }

    #[test]
    fn surjectivity_check_deduplicates_outputs() {
        // Two inputs map to the same output — a genuine surjection.
        let items = parse_items(
            "fn to_axis(d: Dir) -> Axis {
                match d {
                    Dir::North => Axis::Vertical,
                    Dir::South => Axis::Vertical,
                    Dir::East  => Axis::Horizontal,
                    Dir::West  => Axis::Horizontal,
                }
            }",
        );

        let syn::Item::Fn(check_fn) = &items[1] else {
            panic!("expected fn item");
        };

        // The check function body has one arm per *unique* output variant
        // (2: Vertical and Horizontal), not one per input arm (4).
        let syn::Stmt::Expr(Expr::Match(inner), _) = check_fn.block.stmts.first().unwrap() else {
            panic!("expected match inside surjectivity_check");
        };
        assert_eq!(inner.arms.len(), 2, "one arm per unique output variant");
    }

    // -- invalid: no match expression -----------------------------------------

    #[test]
    #[should_panic(expected = "can only be used on functions containing a match expression")]
    fn no_match_panics() {
        run("fn map(l: Letter) -> Letter { l }");
    }

    // -- invalid: arm patterns are not enum variant paths ---------------------

    #[test]
    #[should_panic(expected = "arm pattern must be an enum variant path")]
    fn wildcard_pattern_panics() {
        run("fn map(a: Foo) -> Foo { match a { _ => Foo::A } }");
    }

    #[test]
    #[should_panic(expected = "arm pattern must be an enum variant path")]
    fn literal_pattern_panics() {
        run("fn map(a: Foo) -> Foo { match a { 1 => Foo::A } }");
    }

    // -- invalid: arm bodies are not enum variant paths -----------------------

    #[test]
    #[should_panic(expected = "arm body must be an enum variant path")]
    fn call_expression_body_panics() {
        run("fn map(a: Foo) -> Foo { match a { Foo::A => bar(), Foo::B => Foo::C } }");
    }

    #[test]
    #[should_panic(expected = "arm body must be an enum variant path")]
    fn literal_body_panics() {
        run("fn map(a: Foo) -> Foo { match a { Foo::A => 42 } }");
    }

    // -- invalid: guards are not allowed --------------------------------------

    #[test]
    #[should_panic(expected = "match guards are not supported")]
    fn guard_panics() {
        run("fn map(a: Foo) -> Foo { match a { Foo::A if cond => Foo::B } }");
    }

    // =========================================================================
    // inject tests
    // =========================================================================

    fn run_inject(code: &str) -> TokenStream2 {
        let func: ItemFn = syn::parse_str(code).expect("test input failed to parse");
        impl_inject_macro("inject", &func)
    }

    fn is_compile_error(ts: &TokenStream2) -> bool {
        ts.to_string().contains("compile_error")
    }

    // -- valid: injective mappings --------------------------------------------

    #[test]
    fn inject_bijection_passes() {
        // A bijection is trivially injective.
        let output = run_inject(
            "fn map(l: Letter) -> Letter {
                match l {
                    Letter::A => Letter::D,
                    Letter::B => Letter::C,
                    Letter::C => Letter::B,
                    Letter::D => Letter::A,
                }
            }",
        );
        assert!(!is_compile_error(&output), "bijection should be accepted");
        // Output is just the original function — no companion function needed.
        let items: Vec<syn::Item> = syn::parse_file(&output.to_string()).unwrap().items;
        assert_eq!(items.len(), 1, "inject emits only the original fn");
    }

    #[test]
    fn inject_strict_injection_passes() {
        // Injective but not surjective: SmallEnum maps into a subset of LargeEnum.
        let output = run_inject(
            "fn embed(s: Small) -> Large {
                match s {
                    Small::A => Large::X,
                    Small::B => Large::Y,
                }
            }",
        );
        assert!(!is_compile_error(&output));
    }

    // -- invalid: non-injective mappings --------------------------------------

    #[test]
    fn inject_many_to_one_fails() {
        // Two different inputs produce the same output → not injective.
        let output = run_inject(
            "fn collapse(d: Dir) -> Axis {
                match d {
                    Dir::North => Axis::Vertical,
                    Dir::South => Axis::Vertical,
                    Dir::East  => Axis::Horizontal,
                    Dir::West  => Axis::Horizontal,
                }
            }",
        );
        assert!(is_compile_error(&output), "many-to-one should be rejected");
        assert!(
            output.to_string().contains("not injective"),
            "error should mention injectivity"
        );
    }

    #[test]
    fn inject_error_names_the_duplicate() {
        let output =
            run_inject("fn f(x: Foo) -> Bar { match x { Foo::A => Bar::X, Foo::B => Bar::X } }");
        assert!(is_compile_error(&output));
        assert!(
            output.to_string().contains("Bar :: X"),
            "error should name the duplicate output variant"
        );
    }

    // =========================================================================
    // onto / one_to_one alias tests
    // =========================================================================

    #[test]
    fn onto_is_surject_alias() {
        let func: ItemFn = syn::parse_str(
            "fn map(l: Letter) -> Letter { match l { Letter::A => Letter::D, Letter::B => Letter::C } }"
        ).unwrap();
        let via_surject = impl_surject_macro("surject", &func);
        let via_onto = impl_surject_macro("onto", &func);
        assert_eq!(via_surject.to_string(), via_onto.to_string());
    }

    #[test]
    fn one_to_one_is_inject_alias() {
        let func: ItemFn = syn::parse_str(
            "fn map(l: Letter) -> Letter { match l { Letter::A => Letter::D, Letter::B => Letter::C } }"
        ).unwrap();
        let via_inject = impl_inject_macro("inject", &func);
        let via_one_to_one = impl_inject_macro("one_to_one", &func);
        assert_eq!(via_inject.to_string(), via_one_to_one.to_string());
    }

    // =========================================================================
    // biject tests
    // =========================================================================

    fn run_biject(code: &str) -> TokenStream2 {
        let func: ItemFn = syn::parse_str(code).expect("test input failed to parse");
        impl_biject_macro("biject", &func)
    }

    #[test]
    fn biject_bijection_passes() {
        let output = run_biject(
            "fn map(l: Letter) -> Letter {
                match l {
                    Letter::A => Letter::D,
                    Letter::B => Letter::C,
                    Letter::C => Letter::B,
                    Letter::D => Letter::A,
                }
            }",
        );
        assert!(!is_compile_error(&output));
        let items: Vec<syn::Item> = syn::parse_file(&output.to_string()).unwrap().items;
        assert_eq!(
            items.len(),
            2,
            "biject emits original fn + bijectivity_check fn"
        );
        let syn::Item::Fn(check) = &items[1] else {
            panic!("expected fn")
        };
        assert_eq!(check.sig.ident, "bijectivity_check_map");
    }

    #[test]
    fn biject_surjective_only_fails_injectivity() {
        // Many-to-one: caught at proc macro level (injectivity check).
        let output = run_biject(
            "fn f(d: Dir) -> Axis {
                match d {
                    Dir::North => Axis::Vertical,
                    Dir::South => Axis::Vertical,
                    Dir::East  => Axis::Horizontal,
                    Dir::West  => Axis::Horizontal,
                }
            }",
        );
        assert!(
            is_compile_error(&output),
            "surjective-only should be rejected"
        );
        assert!(output.to_string().contains("not injective"));
    }

    #[test]
    fn biject_injective_only_generates_surjectivity_check() {
        // Strict injection (SmallEnum -> LargeEnum): injectivity passes,
        // but the generated bijectivity_check fn will be non-exhaustive,
        // which the compiler will catch when the output is compiled for real.
        let output = run_biject(
            "fn embed(s: Small) -> Large {
                match s {
                    Small::A => Large::X,
                    Small::B => Large::Y,
                }
            }",
        );
        // No compile_error in the token stream itself — the surjectivity
        // failure will be a compiler error when the output is compiled.
        assert!(!is_compile_error(&output));
        let items: Vec<syn::Item> = syn::parse_file(&output.to_string()).unwrap().items;
        assert_eq!(items.len(), 2);
        let syn::Item::Fn(check) = &items[1] else {
            panic!()
        };
        assert_eq!(check.sig.ident, "bijectivity_check_embed");
    }
}
