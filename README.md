# surject

Compile-time verification of **surjective**, **injective**, and **bijective**
properties on enum-to-enum `match` expressions.

## Background

When translating between two enum types with a `match` expression it is easy
to accidentally:

* leave an output variant unreachable (not **surjective** / not *onto*), or
* map two different inputs to the same output (not **injective** / not
  *one-to-one*).

`surject` catches both mistakes **at compile time** through proc-macro
attributes you place on the mapping function.

## Macros

| Attribute | Alias | Property enforced |
|---|---|---|
| `#[surject]` | `#[onto]` | Every output variant is produced by at least one arm (*onto*). |
| `#[inject]` | `#[one_to_one]` | No two arms produce the same output variant (*one-to-one*). |
| `#[biject]` | — | Both of the above simultaneously (*bijection*). |

---

## `#[surject]` / `#[onto]`

Verifies that every variant of the output enum is reachable — i.e. the mapping
is *onto* (surjective).

The macro generates a private companion function
`surjectivity_check_<fn_name>` that exhaustively matches over the output type
using only the variants seen in the annotated function's arms.  If any variant
is absent, the **compiler** reports a non-exhaustive pattern error pointing at
the `#[surject]` attribute.

### Compile-pass: many-to-one mapping

A classic surjection — multiple inputs collapse to the same output variant, but
every output variant is covered:

```rust
use surject::surject;

enum Direction { North, South, East, West }
enum Axis      { Vertical, Horizontal }

#[surject]
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
use surject::surject;

enum Letter { A, B, C, D }

#[surject]
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
use surject::surject;

enum Direction { North, South, East, West }
enum Axis      { Vertical, Horizontal }

#[surject]
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

## `#[inject]` / `#[one_to_one]`

Verifies that no two arms produce the same output variant — i.e. the mapping is
*one-to-one* (injective).

Unlike `#[surject]`, this does **not** require all output variants to be
covered.  An injection from a smaller domain into a larger codomain — where some
output variants are legitimately never produced — is perfectly valid.

The duplicate check runs **at macro-expansion time** and emits a span-accurate
error pointing at the second occurrence of the duplicate output path.

### Compile-pass: strict injection (smaller domain)

`SmallEnum` embeds into a subset of `LargeEnum`.  `LargeEnum::Z` is never
produced — that is fine:

```rust
use surject::inject;

enum SmallEnum { A, B }
enum LargeEnum { X, Y, Z }

#[inject]
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
use surject::inject;

enum Letter { A, B, C, D }

#[inject]
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
use surject::inject;

enum Direction { North, South, East, West }
enum Axis      { Vertical, Horizontal }

#[inject]
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

## `#[biject]`

Verifies both properties simultaneously: no duplicate outputs **and** every
output variant is covered.

The injectivity check runs first at expansion time.  If it passes, a
`bijectivity_check_<fn_name>` companion function is generated to let the
compiler verify exhaustiveness.

### Compile-pass: true bijection

```rust
use surject::biject;

enum Letter { A, B, C, D }

#[biject]
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
use surject::biject;

enum Direction { North, South, East, West }
enum Axis      { Vertical, Horizontal }

#[biject]
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
use surject::biject;

enum SmallEnum { A, B }
enum LargeEnum { X, Y, Z }

#[biject]
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
surject = { path = "..." }   # or version = "..." once published
```

## License

Licensed under the [MIT License](LICENSE).
