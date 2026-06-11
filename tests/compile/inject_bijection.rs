use surject::inject;

// A bijection: every input maps to a distinct output, so it is injective.
enum Letter {
    A,
    B,
    C,
    D,
}

#[inject]
fn map(l: Letter) -> Letter {
    match l {
        Letter::A => Letter::D,
        Letter::B => Letter::C,
        Letter::C => Letter::B,
        Letter::D => Letter::A,
    }
}

fn main() {}
