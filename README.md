# bijective

Compile-time verification of **surjective**, **injective**, and **bijective**
properties on enum-to-enum `match` expressions.

## Background

From a mathematical point of view, match expressions in Rust are [total functions](https://en.wikipedia.org/wiki/Partial_function)
by design: The compiler ensures every possible value in the domain (every
pattern) is handled.

For most use cases, this is the most that's required and/or feasible. However, in
simple enum-to-enum mappings (for example, `From` implementations of enums that
cross architecture boundaries) it is often desirable to have stricter assurances.

When translating between two enum types with a `match` expression it is easy to
accidentally:

* leave an output variant unreachable (not **surjective**), or
* map two different inputs to the same output (not **injective**).

`bijective` provides drop-in attribute macros to enforce compile time checks for
these mistakes, by annotating `let _ = match {}` bindings or functions returning
a match. Support for attributes on expressions is still unstable so only these
two use cases are currently provided.

## The three macros

| Attribute       | Alias           | Property enforced                                                |
|-----------------|-----------------|------------------------------------------------------------------|
| `#[surjective]` | `#[onto]`       | Every output variant is produced by at least one arm (**onto**). |
| `#[injective]`  | `#[one_to_one]` | No two arms produce the same output variant (**one-to-one**).    |
| `#[bijective]`  | —               | Both of the above simultaneously (**bijection**).                |

## Usage

```rust
use bijective::{surjective, injective, bijective};

enum Direction { North, South, East, West }
enum Axis      { Vertical, Horizontal }

// OK — every Axis variant is produced at least once.
#[surjective]
fn to_axis(d: Direction) -> Axis {
    match d {
        Direction::North => Axis::Vertical,
        Direction::South => Axis::Vertical,
        Direction::East  => Axis::Horizontal,
        Direction::West  => Axis::Horizontal,
    }
}

enum Small { A, B }
enum Large { X, Y, Z }

// OK — every Small variant maps to a *distinct* Large variant.
#[injective]
fn embed(s: Small) -> Large {
    match s {
        Small::A => Large::X,
        Small::B => Large::Y,
    }
}

enum Letter { A, B, C, D }
enum Number { One, Two, Three, Four }

// OK — every letter maps to a distinct number, and all number variants appear as output.
#[bijective]
fn swap(l: Letter) -> Number {
    match l {
        Letter::A => Number::One,
        Letter::B => Number::Two,
        Letter::C => Number::Three,
        Letter::D => Number::Four,
    }
}
```

## How the checks work

### Surjectivity (`#[surjective]`, `#[onto]`)

The macro generates a private companion function `surjectivity_check_<fn_name>`
whose body is a `match` over the *output* type covering every unique variant that
appears as an arm body. If any variant of the output enum is absent, the compiler
reports a **non-exhaustive pattern** error pointing at the `#[surjective]`
attribute. 

This type of manual hack has existed for years in Rust folklore, but it
is ugly and verbose, and users first encountering it usually experience a mixture
of awe and horror. 

The trick is legit though: because the function is `dead_code`,
it has 0 runtime cost, and ensures the verification happens at compile time, which
is usually preferable to tests. Abstracting away this trick, and replacing it with
a single line drop-in attribute has been the main motivator for this crate. 
It makes intent clear and concise in a way a preimage closure with comments can't
match.

### Injectivity (`#[injective]`, `#[one_to_one]`)

The macro inspects every arm at expansion time and emits a **`compile_error!`**
with a span pointing at the duplicate output variant if the same output path
appears more than once.

### Bijectivity (`#[bijective]`)

Combines both checks: the injectivity check runs first (at expansion time), and
the surjectivity check is delegated to the compiler via generated code.

## Constraints and limitations

A mapping function must satisfy all of the following for the attribute to accept it:

* The function body must contain a `match` expression. If there are several,
  only the **outermost** one is analysed.
* Every arm **pattern** must be a plain enum-variant path
  (e.g. `Enum::Variant`). Wildcards (`_`), literals, tuple-struct patterns,
  and `or`-patterns are not supported.
* Every arm **body** must be a plain enum-variant path (e.g. `Enum::Variant`).
  Arbitrary expressions, function calls, and struct literals are not supported.
* Match **guards** (`if condition`) are not supported.
* The checks are purely syntactic: the macro does not resolve types, so it
  cannot detect if two syntactically different paths refer to the same variant
  via `use` aliases.
* The surjectivity failure error is not extremely clear. This is due to the way
  it is implemented, where the compiler actually sees a missing arm in a `match`
  expression of the generated code, so we can't override that error during
  macro expansion.

## AI disclosure

LLMs (agents, edit predictions) have been used during the development of this
crate's code. Intent, design and implementation have all been human-driven, and
I have read all code and refactored most of it to my own personal liking.

All prose and documentation are my own personal words, and I advocate that others
do the same. I’m ok with machines reading machine generated slop, but text made
for humans is best written by other humans.

Commits with AI assistance have an `Assisted-by` footer in the [NixOS style](https://github.com/NixOS/nixpkgs/blob/master/CONTRIBUTING.md#transparency)

## License

Licensed under the [MIT License](LICENSE).
