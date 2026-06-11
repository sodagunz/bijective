use crate::validation::{
    AsExprPath, check_injectivity, enum_type_of_expr, find_and_validate, surjectivity_check_arms,
};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::ItemFn;

pub(crate) fn impl_surject_macro(attr: &str, func: &ItemFn) -> TokenStream2 {
    let arms = find_and_validate(attr, func);

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

pub(crate) fn impl_inject_macro(attr: &str, func: &ItemFn) -> TokenStream2 {
    let arms = find_and_validate(attr, func);

    if let Some(err) = check_injectivity(arms) {
        return err;
    }

    quote! { #func }
}

pub(crate) fn impl_biject_macro(attr: &str, func: &ItemFn) -> TokenStream2 {
    let arms = find_and_validate(attr, func);

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
