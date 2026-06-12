# bijective

Compile-time verification of **surjective**, **injective**, and **bijective**
properties on enum-to-enum `match` expressions.

## Background

When translating between two enum types with a `match` expression it is easy
to accidentally:

* leave an output variant unreachable (not **surjective** / not *onto*), or
* map two different inputs to the same output (not **injective** / not
  *one-to-one*).

`bijective` catches both mistakes **at compile time** through proc-macro
attributes you place on the mapping function.

## Macros

| Attribute | Alias | Property enforced |
|---|---|---|
| `#[surjective]` | `#[onto]` | Every output variant is produced by at least one arm (*onto*). |
| `#[injective]` | `#[one_to_one]` | No two arms produce the same output variant (*one-to-one*). |
| `#[bijective]` | — | Both of the above simultaneously (*bijection*). |

---

## `#[surjective]` / `#[onto]`

Verifies that every variant of the output enum is reachable — i.e. the mapping
is *onto* (surjective).

The macro generates a private companion function
`surjectivity_check_<fn_name>` that exhaustively matches over the output type
using only the variants seen in the annotated function's arms.  If any variant
is absent, the **compiler** reports a non-exhaustive pattern error pointing at
the `#[surjective]` attribute.

### Compile-pass: many-to-one mapping

A classic surjection — multiple inputs collapse to the same output variant, but
every output variant is covered:

```rust
use bijective::surjective;

enum Direction { North, South, East, West }
enum Axis      { Vertical, Horizontal }

#[surjective]
fn to_axis(d: Direction) -> Axis {
    match d {
        Direction::North => Axis::Vertical,
        Direction::South => Axis::Vertical,
        Direction::East  => Axis::Horizontal,
        Direction::West  => Axis::Horizontal,
    }
}
```

### Compile-pass: bijection

A bijection is also surjective (every output appears exactly once):

```rust
use bijective::surjective;

enum Letter { A, B, C, D }

#[surjective]
fn swap(l: Letter) -> Letter {
    match l {
        Letter::A => Letter::D,
        Letter::B => Letter::C,
        Letter::C => Letter::B,
        Letter::D => Letter::A,
    }
}
```

### Compile-fail: missing output variant

`Axis::Horizontal` is never produced — the compiler rejects this:

```rust,compile_fail
use bijective::surjective;

enum Direction { North, South, East, West }
enum Axis      { Vertical, Horizontal }

#[surjective]
fn to_axis(d: Direction) -> Axis {
    match d {
        Direction::North => Axis::Vertical,
        Direction::South => Axis::Vertical,
        Direction::East  => Axis::Vertical,  // should be Axis::Horizontal
        Direction::West  => Axis::Vertical,
    }
}
// error[E0004]: non-exhaustive patterns: `Axis::Horizontal` not covered
```

---

## `#[injective]` / `#[one_to_one]`

Verifies that no two arms produce the same output variant — i.e. the mapping is
*one-to-one* (injective).

Unlike `#[surjective]`, this does **not** require all output variants to be
covered.  An injection from a smaller domain into a larger codomain — where some
output variants are legitimately never produced — is perfectly valid.

The duplicate check runs **at macro-expansion time** and emits a span-accurate
error pointing at the second occurrence of the duplicate output path.

### Compile-pass: strict injection (smaller domain)

`SmallEnum` embeds into a subset of `LargeEnum`.  `LargeEnum::Z` is never
produced — that is fine:

```rust
use bijective::injective;

enum SmallEnum { A, B }
enum LargeEnum { X, Y, Z }

#[injective]
fn embed(s: SmallEnum) -> LargeEnum {
    match s {
        SmallEnum::A => LargeEnum::X,
        SmallEnum::B => LargeEnum::Y,
    }
}
```

### Compile-pass: bijection

A bijection is also injective (each output appears exactly once):

```rust
use bijective::injective;

enum Letter { A, B, C, D }

#[injective]
fn swap(l: Letter) -> Letter {
    match l {
        Letter::A => Letter::D,
        Letter::B => Letter::C,
        Letter::C => Letter::B,
        Letter::D => Letter::A,
    }
}
```

### Compile-fail: duplicate output variant

Both `North` and `South` produce `Axis::Vertical`:

```rust,compile_fail
use bijective::injective;

enum Direction { North, South, East, West }
enum Axis      { Vertical, Horizontal }

#[injective]
fn to_axis(d: Direction) -> Axis {
    match d {
        Direction::North => Axis::Vertical,
        Direction::South => Axis::Vertical,  // error here
        Direction::East  => Axis::Horizontal,
        Direction::West  => Axis::Horizontal,
    }
}
// error: inject: `Axis :: Vertical` is produced by more than one arm;
//        the mapping is not injective
```

---

## `#[bijective]`

Verifies both properties simultaneously: no duplicate outputs **and** every
output variant is covered.

The injectivity check runs first at expansion time.  If it passes, a
`bijectivity_check_<fn_name>` companion function is generated to let the
compiler verify exhaustiveness.

### Compile-pass: true bijection

```rust
use bijective::bijective;

enum Letter { A, B, C, D }

#[bijective]
fn swap(l: Letter) -> Letter {
    match l {
        Letter::A => Letter::D,
        Letter::B => Letter::C,
        Letter::C => Letter::B,
        Letter::D => Letter::A,
    }
}
```

### Compile-fail: not injective (duplicate output)

`Axis::Vertical` appears twice — the injectivity check fires at expansion time:

```rust,compile_fail
use bijective::bijective;

enum Direction { North, South, East, West }
enum Axis      { Vertical, Horizontal }

#[bijective]
fn to_axis(d: Direction) -> Axis {
    match d {
        Direction::North => Axis::Vertical,
        Direction::South => Axis::Vertical,  // error: not injective
        Direction::East  => Axis::Horizontal,
        Direction::West  => Axis::Horizontal,
    }
}
```

### Compile-fail: not surjective (missing output variant)

`LargeEnum::Z` is never produced — the compiler's exhaustiveness check fires:

```rust,compile_fail
use bijective::bijective;

enum SmallEnum { A, B }
enum LargeEnum { X, Y, Z }

#[bijective]
fn embed(s: SmallEnum) -> LargeEnum {
    match s {
        SmallEnum::A => LargeEnum::X,
        SmallEnum::B => LargeEnum::Y,
    }
}
// error[E0004]: non-exhaustive patterns: `LargeEnum::Z` not covered
```

---

## Constraints and limitations

The macros apply **syntactic** checks only.  A mapping function must satisfy
all of the following for the attribute to accept it:

* The function body must contain a `match` expression.  If there are several,
  only the **outermost** one is analysed.
* Every arm **pattern** must be a plain enum-variant path
  (e.g. `Enum::Variant`).  Wildcards (`_`), literals, tuple-struct patterns,
  and `or`-patterns are not supported.
* Every arm **body** must be a plain enum-variant path (e.g. `Enum::Variant`).
  Arbitrary expressions, function calls, and struct literals are not supported.
* Match **guards** (`if condition`) are not supported.
* The checks are purely syntactic: the macro does not resolve types, so it
  cannot detect if two syntactically different paths refer to the same variant
  via `use` aliases.

## Dependency

```toml
[dependencies]
bijective = { path = "..." }   # or version = "..." once published
```

## License

Licensed under the [MIT License](LICENSE).
