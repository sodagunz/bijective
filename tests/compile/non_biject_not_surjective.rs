use surject::biject;

// Injective but not surjective: LargeEnum::Z is never produced.
// The surjectivity check (compiler exhaustiveness) should reject this.
enum SmallEnum {
    A,
    B,
}
enum LargeEnum {
    X,
    Y,
    Z,
}

#[biject]
fn embed(s: SmallEnum) -> LargeEnum {
    match s {
        SmallEnum::A => LargeEnum::X,
        SmallEnum::B => LargeEnum::Y,
    }
}

fn main() {}
