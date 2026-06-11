use crate::implementation::{impl_biject_macro, impl_inject_macro, impl_surject_macro};
use proc_macro2::TokenStream as TokenStream2;
use syn::ItemFn;

fn run(code: &str) -> TokenStream2 {
    let func: ItemFn = syn::parse_str(code).expect("test input failed to parse");
    impl_surject_macro("surject", &func)
}

fn run_inject(code: &str) -> TokenStream2 {
    let func: ItemFn = syn::parse_str(code).expect("test input failed to parse");
    impl_inject_macro("inject", &func)
}

fn run_biject(code: &str) -> TokenStream2 {
    let func: ItemFn = syn::parse_str(code).expect("test input failed to parse");
    impl_biject_macro("biject", &func)
}

fn parse_items(code: &str) -> Vec<syn::Item> {
    let output = run(code);
    println!("{output}");
    syn::parse_file(&output.to_string())
        .expect("output should be valid items")
        .items
}

fn is_compile_error(ts: &TokenStream2) -> bool {
    ts.to_string().contains("compile_error")
}

// -- surject ------------------------------------------------------------------

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

    let syn::Stmt::Expr(syn::Expr::Match(inner), _) = check_fn.block.stmts.first().unwrap() else {
        panic!("expected match inside surjectivity_check");
    };
    assert_eq!(inner.arms.len(), 2, "one arm per unique output variant");
}

#[test]
#[should_panic(expected = "can only be used on functions containing a match expression")]
fn no_match_panics() {
    run("fn map(l: Letter) -> Letter { l }");
}

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

#[test]
#[should_panic(expected = "match guards are not supported")]
fn guard_panics() {
    run("fn map(a: Foo) -> Foo { match a { Foo::A if cond => Foo::B } }");
}

// -- inject -------------------------------------------------------------------

#[test]
fn inject_bijection_passes() {
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
    let items: Vec<syn::Item> = syn::parse_file(&output.to_string()).unwrap().items;
    assert_eq!(items.len(), 1, "inject emits only the original fn");
}

#[test]
fn inject_strict_injection_passes() {
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

#[test]
fn inject_many_to_one_fails() {
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
    assert!(output.to_string().contains("not injective"));
}

#[test]
fn inject_error_names_the_duplicate() {
    let output =
        run_inject("fn f(x: Foo) -> Bar { match x { Foo::A => Bar::X, Foo::B => Bar::X } }");
    assert!(is_compile_error(&output));
    assert!(output.to_string().contains("Bar :: X"));
}

// -- aliases ------------------------------------------------------------------

#[test]
fn onto_is_surject_alias() {
    let func: ItemFn = syn::parse_str(
        "fn map(l: Letter) -> Letter { match l { Letter::A => Letter::D, Letter::B => Letter::C } }"
    ).unwrap();
    assert_eq!(
        impl_surject_macro("surject", &func).to_string(),
        impl_surject_macro("onto", &func).to_string(),
    );
}

#[test]
fn one_to_one_is_inject_alias() {
    let func: ItemFn = syn::parse_str(
        "fn map(l: Letter) -> Letter { match l { Letter::A => Letter::D, Letter::B => Letter::C } }"
    ).unwrap();
    assert_eq!(
        impl_inject_macro("inject", &func).to_string(),
        impl_inject_macro("one_to_one", &func).to_string(),
    );
}

// -- biject -------------------------------------------------------------------

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
    let output = run_biject(
        "fn embed(s: Small) -> Large {
            match s {
                Small::A => Large::X,
                Small::B => Large::Y,
            }
        }",
    );
    assert!(!is_compile_error(&output));
    let items: Vec<syn::Item> = syn::parse_file(&output.to_string()).unwrap().items;
    assert_eq!(items.len(), 2);
    let syn::Item::Fn(check) = &items[1] else {
        panic!()
    };
    assert_eq!(check.sig.ident, "bijectivity_check_embed");
}
