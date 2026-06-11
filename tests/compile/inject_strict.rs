use surject::inject;

// A strict injection: SmallEnum embeds into a subset of LargeEnum.
// Not surjective (LargeEnum::Z is never produced), but still injective.
enum SmallEnum {
    A,
    B,
}
enum LargeEnum {
    X,
    Y,
    Z,
}

#[inject]
fn embed(s: SmallEnum) -> LargeEnum {
    match s {
        SmallEnum::A => LargeEnum::X,
        SmallEnum::B => LargeEnum::Y,
    }
}

fn main() {}
