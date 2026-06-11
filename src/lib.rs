mod implementation;
#[cfg(test)]
mod tests;
mod validation;
mod visitor;

use implementation::{impl_biject_macro, impl_inject_macro, impl_surject_macro};
use proc_macro::TokenStream;
use syn::ItemFn;

/// Verifies at compile time that the annotated function's match expression is
/// **surjective** (onto): every variant of the output enum is produced by at
/// least one arm.
#[proc_macro_attribute]
pub fn surject(attr: TokenStream, input: TokenStream) -> TokenStream {
    let func: ItemFn = syn::parse(input).unwrap();
    impl_surject_macro(&attr.to_string(), &func).into()
}

/// Alias for [`#[surject]`](macro@surject).
#[proc_macro_attribute]
pub fn onto(attr: TokenStream, input: TokenStream) -> TokenStream {
    surject(attr, input)
}

/// Verifies at compile time that the annotated function's match expression is
/// **injective** (one-to-one): no two arms produce the same output variant.
#[proc_macro_attribute]
pub fn inject(attr: TokenStream, input: TokenStream) -> TokenStream {
    let func: ItemFn = syn::parse(input).unwrap();
    impl_inject_macro(&attr.to_string(), &func).into()
}

/// Alias for [`#[inject]`](macro@inject).
#[proc_macro_attribute]
pub fn one_to_one(attr: TokenStream, input: TokenStream) -> TokenStream {
    inject(attr, input)
}

/// Verifies at compile time that the annotated function's match expression is
/// **bijective**: both injective and surjective simultaneously.
#[proc_macro_attribute]
pub fn biject(attr: TokenStream, input: TokenStream) -> TokenStream {
    let func: ItemFn = syn::parse(input).unwrap();
    impl_biject_macro(&attr.to_string(), &func).into()
}
