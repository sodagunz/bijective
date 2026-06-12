//! # bijective
//!
//! Compile-time verification of **surjective**, **injective**, and **bijective**
//! properties on enum-to-enum `match` expressions.
//!
//! ## Background
//!
//! When translating between two enum types with a `match` expression it is easy
//! to accidentally:
//!
//! * leave an output variant unreachable (not **surjective** / not *onto*), or
//! * map two different inputs to the same output (not **injective** / not
//!   *one-to-one*).
//!
//! `bijective` catches both mistakes **at compile time** through proc-macro
//! attributes that you place on the mapping function.
//!
//! ## The three macros
//!
//! | Attribute                     | Alias           | Property enforced                                                |
//! |-------------------------------|-----------------|------------------------------------------------------------------|
//! | [`#[surjective]`](macro@surjective) | `#[onto]`       | Every output variant is produced by at least one arm (**onto**). |
//! | [`#[injective]`](macro@injective)   | `#[one_to_one]` | No two arms produce the same output variant (**one-to-one**).    |
//! | [`#[bijective]`](macro@bijective)   | —               | Both of the above simultaneously (**bijection**).                |
//!
//! ## Quick start
//!
//! ```rust
//! use bijective::{surjective, injective, bijective};
//!
//! enum Direction { North, South, East, West }
//! enum Axis      { Vertical, Horizontal }
//!
//! // OK — every Axis variant is produced at least once.
//! #[surjective]
//! fn to_axis(d: Direction) -> Axis {
//!     match d {
//!         Direction::North => Axis::Vertical,
//!         Direction::South => Axis::Vertical,
//!         Direction::East  => Axis::Horizontal,
//!         Direction::West  => Axis::Horizontal,
//!     }
//! }
//!
//! enum Small { A, B }
//! enum Large { X, Y, Z }
//!
//! // OK — every Small variant maps to a *distinct* Large variant.
//! #[injective]
//! fn embed(s: Small) -> Large {
//!     match s {
//!         Small::A => Large::X,
//!         Small::B => Large::Y,
//!     }
//! }
//!
//! enum Letter { A, B, C, D }
//!
//! // OK — every letter maps to a distinct letter, and all letters appear as output.
//! #[bijective]
//! fn swap(l: Letter) -> Letter {
//!     match l {
//!         Letter::A => Letter::D,
//!         Letter::B => Letter::C,
//!         Letter::C => Letter::B,
//!         Letter::D => Letter::A,
//!     }
//! }
//! ```
//!
//! ## How the checks work
//!
//! ### Surjectivity (`#[surjective]`, `#[onto]`)
//!
//! The macro generates a private companion function
//! `surjectivity_check_<fn_name>` whose body is a `match` over the *output*
//! type covering every unique variant that appears as an arm body.  If any
//! variant of the output enum is absent, the compiler reports a
//! **non-exhaustive pattern** error pointing at the `#[surjective]` attribute.
//!
//! ### Injectivity (`#[injective]`, `#[one_to_one]`)
//!
//! The macro inspects every arm at expansion time and emits a
//! **`compile_error!`** with a span pointing at the second (duplicate) output
//! variant if the same output path appears more than once.
//!
//! ### Bijectivity (`#[bijective]`)
//!
//! Combines both checks: the injectivity check runs first (at expansion time),
//! and the surjectivity check is delegated to the compiler via a generated
//! `bijectivity_check_<fn_name>` companion function.
//!
//! ## Constraints and limitations
//!
//! The macros apply **syntactic** checks only.  A mapping function must satisfy
//! all of the following for the attribute to accept it:
//!
//! * The function body must contain a `match` expression.  If there are several,
//!   only the **outermost** one is analysed.
//! * Every arm **pattern** must be a plain enum-variant path
//!   (e.g. `Enum::Variant`).  Wildcards (`_`), literals, tuple-struct patterns,
//!   and `or`-patterns are not supported.
//! * Every arm **body** must be a plain enum-variant path
//!   (e.g. `Enum::Variant`).  Arbitrary expressions, function calls, and struct
//!   literals are not supported.
//! * Match **guards** (`if condition`) are not supported.
//! * The checks are purely syntactic: the macro does not resolve types, so it
//!   cannot detect if two syntactically different paths refer to the same variant
//!   via `use` aliases.

mod implementation;
#[cfg(test)]
mod tests;
mod validation;
mod visitor;

use implementation::{impl_bijective_macro, impl_injective_macro, impl_surjective_macro};
use proc_macro::TokenStream;
use syn::ItemFn;

/// Verifies at compile time that the annotated function's `match` expression
/// is **surjective** (*onto*): every variant of the output enum is produced by
/// at least one arm.
///
/// This is an alias for [`#[onto]`](macro@onto).
///
/// # How it works
///
/// The macro generates a private companion function
/// `surjectivity_check_<fn_name>` that exhaustively matches over the *output*
/// type.  If any variant of that type is absent from the original function's
/// arms, the compiler reports a non-exhaustive pattern error pointing at the
/// `#[surjective]` attribute site.
///
/// # Panics
///
/// Panics at compile time if the annotated item is not a syntactically valid
/// function, or if the function violates the structural requirements below.
///
/// # Requirements
///
/// * The function body must contain a `match` expression.
/// * Every arm pattern must be a plain enum-variant path (`Enum::Variant`).
/// * Every arm body must be a plain enum-variant path (`Enum::Variant`).
/// * Match guards are not supported.
///
/// # Examples
///
/// ## Compile-pass: many-to-one mapping
///
/// A classic surjection — multiple inputs share an output variant, but every
/// output variant is covered:
///
/// ```rust,ignore
/// use bijective::surjective;
///
/// enum Direction { North, South, East, West }
/// enum Axis      { Vertical, Horizontal }
///
/// #[surjective]
/// fn to_axis(d: Direction) -> Axis {
///     match d {
///         Direction::North => Axis::Vertical,
///         Direction::South => Axis::Vertical,
///         Direction::East  => Axis::Horizontal,
///         Direction::West  => Axis::Horizontal,
///     }
/// }
/// ```
///
/// ## Compile-pass: bijection
///
/// A bijection is also surjective (every output appears exactly once):
///
/// ```rust,ignore
/// use bijective::surjective;
///
/// enum Letter { A, B, C, D }
///
/// #[surjective]
/// fn swap(l: Letter) -> Letter {
///     match l {
///         Letter::A => Letter::D,
///         Letter::B => Letter::C,
///         Letter::C => Letter::B,
///         Letter::D => Letter::A,
///     }
/// }
/// ```
///
/// ## Compile-fail: missing output variant
///
/// `Axis::Horizontal` is never produced, so the compiler rejects this:
///
/// ```rust,ignore
/// use bijective::surjective;
///
/// enum Direction { North, South, East, West }
/// enum Axis      { Vertical, Horizontal }
///
/// #[surjective]                          // error[E0004]: non-exhaustive patterns:
/// fn to_axis(d: Direction) -> Axis {  //   `Axis::Horizontal` not covered
///     match d {
///         Direction::North => Axis::Vertical,
///         Direction::South => Axis::Vertical,
///         Direction::East  => Axis::Vertical,
///         Direction::West  => Axis::Vertical,
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn surjective(attr: TokenStream, input: TokenStream) -> TokenStream {
    let func: ItemFn = syn::parse(input).unwrap();
    impl_surjective_macro(&attr.to_string(), &func).into()
}

/// Alias for [`#[surjective]`](macro@surjective).
///
/// Use `#[onto]` when you prefer the set-theory terminology (*onto* = every
/// element of the codomain is mapped to by at least one element of the domain).
///
/// # Example
///
/// ```rust,ignore
/// use bijective::onto;
///
/// enum Direction { North, South, East, West }
/// enum Axis      { Vertical, Horizontal }
///
/// #[onto]
/// fn to_axis(d: Direction) -> Axis {
///     match d {
///         Direction::North => Axis::Vertical,
///         Direction::South => Axis::Vertical,
///         Direction::East  => Axis::Horizontal,
///         Direction::West  => Axis::Horizontal,
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn onto(attr: TokenStream, input: TokenStream) -> TokenStream {
    surjective(attr, input)
}

/// Verifies at compile time that the annotated function's `match` expression
/// is **injective** (*one-to-one*): no two arms produce the same output
/// variant.
///
/// This is an alias for [`#[one_to_one]`](macro@one_to_one).
///
/// # How it works
///
/// At macro-expansion time the macro collects every arm body and emits a
/// span-accurate `compile_error!` pointing at the *second* occurrence of any
/// duplicate output variant.
///
/// Note that `#[injective]` does **not** require the mapping to be surjective.
/// An injective mapping from a smaller domain to a larger codomain — where some
/// output variants are legitimately never produced — is perfectly valid.
///
/// # Panics
///
/// Panics at compile time if the annotated item is not a syntactically valid
/// function, or if the function violates the structural requirements below.
///
/// # Requirements
///
/// * The function body must contain a `match` expression.
/// * Every arm pattern must be a plain enum-variant path (`Enum::Variant`).
/// * Every arm body must be a plain enum-variant path (`Enum::Variant`).
/// * Match guards are not supported.
///
/// # Examples
///
/// ## Compile-pass: strict injection (smaller domain)
///
/// `SmallEnum` embeds into a subset of `LargeEnum`.  `LargeEnum::Z` is never
/// produced — that is fine, because `#[injective]` only cares about uniqueness:
///
/// ```rust
/// use bijective::injective;
///
/// enum SmallEnum { A, B }
/// enum LargeEnum { X, Y, Z }
///
/// #[injective]
/// fn embed(s: SmallEnum) -> LargeEnum {
///     match s {
///         SmallEnum::A => LargeEnum::X,
///         SmallEnum::B => LargeEnum::Y,
///     }
/// }
/// ```
///
/// ## Compile-pass: bijection
///
/// A bijection is also injective (each output appears exactly once):
///
/// ```rust
/// use bijective::injective;
///
/// enum Letter { A, B, C, D }
///
/// #[injective]
/// fn swap(l: Letter) -> Letter {
///     match l {
///         Letter::A => Letter::D,
///         Letter::B => Letter::C,
///         Letter::C => Letter::B,
///         Letter::D => Letter::A,
///     }
/// }
/// ```
///
/// ## Compile-fail: duplicate output variant
///
/// Both `North` and `South` produce `Axis::Vertical`, so the mapping is not
/// injective:
///
/// ```rust,ignore
/// use bijective::injective;
///
/// enum Direction { North, South, East, West }
/// enum Axis      { Vertical, Horizontal }
///
/// #[injective]
/// fn to_axis(d: Direction) -> Axis {
///     match d {
///         Direction::North => Axis::Vertical,
///         Direction::South => Axis::Vertical,  // error: `Axis::Vertical` produced
///         Direction::East  => Axis::Horizontal, //        by more than one arm
///         Direction::West  => Axis::Horizontal,
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn injective(attr: TokenStream, input: TokenStream) -> TokenStream {
    let func: ItemFn = syn::parse(input).unwrap();
    impl_injective_macro(&attr.to_string(), &func).into()
}

/// Alias for [`#[injective]`](macro@injective).
///
/// Use `#[one_to_one]` when you prefer the plain English terminology over the
/// mathematical *inject*.
///
/// # Example
///
/// ```rust
/// use bijective::one_to_one;
///
/// enum SmallEnum { A, B }
/// enum LargeEnum { X, Y, Z }
///
/// #[one_to_one]
/// fn embed(s: SmallEnum) -> LargeEnum {
///     match s {
///         SmallEnum::A => LargeEnum::X,
///         SmallEnum::B => LargeEnum::Y,
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn one_to_one(attr: TokenStream, input: TokenStream) -> TokenStream {
    injective(attr, input)
}

/// Verifies at compile time that the annotated function's `match` expression
/// is **bijective**: both **injective** (no duplicate outputs) and
/// **surjective** (every output variant is covered).
///
/// # How it works
///
/// 1. **Injectivity** is checked at macro-expansion time: a span-accurate
///    `compile_error!` is emitted for the first duplicate output variant found.
/// 2. **Surjectivity** is delegated to the compiler via an exhaustive
///    `bijectivity_check_<fn_name>` companion function.  A missing output
///    variant causes a non-exhaustive pattern error pointing at the `#[bijective]`
///    attribute site.
///
/// The injectivity check runs first; if it fails the surjectivity helper is not
/// generated.
///
/// # Requirements
///
/// * The function body must contain a `match` expression.
/// * Every arm pattern must be a plain enum-variant path (`Enum::Variant`).
/// * Every arm body must be a plain enum-variant path (`Enum::Variant`).
/// * Match guards are not supported.
/// * For the bijection to be well-typed the input and output enums must have
///   the same number of variants (though this is not explicitly checked — the
///   combined injectivity + surjectivity constraints enforce it indirectly).
///
/// # Panics
///
/// Panics at compile time if the annotated item is not a syntactically valid
/// function, or if the function violates the structural requirements above.
///
/// # Examples
///
/// ## Compile-pass: true bijection
///
/// Every letter maps to a distinct letter, and all letters appear as output:
///
/// ```rust
/// use bijective::bijective;
///
/// enum Letter { A, B, C, D }
///
/// #[bijective]
/// fn swap(l: Letter) -> Letter {
///     match l {
///         Letter::A => Letter::D,
///         Letter::B => Letter::C,
///         Letter::C => Letter::B,
///         Letter::D => Letter::A,
///     }
/// }
/// ```
///
/// ## Compile-fail: not injective (duplicate output)
///
/// `Axis::Vertical` appears twice, so the injectivity check rejects this at
/// expansion time:
///
/// ```rust,ignore
/// use bijective::bijective;
///
/// enum Direction { North, South, East, West }
/// enum Axis      { Vertical, Horizontal }
///
/// #[bijective]
/// fn to_axis(d: Direction) -> Axis {
///     match d {
///         Direction::North => Axis::Vertical,
///         Direction::South => Axis::Vertical,  // error: not injective
///         Direction::East  => Axis::Horizontal,
///         Direction::West  => Axis::Horizontal,
///     }
/// }
/// ```
///
/// ## Compile-fail: not surjective (missing output variant)
///
/// `LargeEnum::Z` is never produced, so the compiler's exhaustiveness check
/// rejects this:
///
/// ```rust,ignore
/// use bijective::bijective;
///
/// enum SmallEnum { A, B }
/// enum LargeEnum { X, Y, Z }
///
/// #[bijective]                                // error[E0004]: non-exhaustive patterns:
/// fn embed(s: SmallEnum) -> LargeEnum {   //   `LargeEnum::Z` not covered
///     match s {
///         SmallEnum::A => LargeEnum::X,
///         SmallEnum::B => LargeEnum::Y,
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn bijective(attr: TokenStream, input: TokenStream) -> TokenStream {
    let func: ItemFn = syn::parse(input).unwrap();
    impl_bijective_macro(&attr.to_string(), &func).into()
}
