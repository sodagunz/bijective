use bijective::injective;

// A bijection: every input maps to a distinct output, so it is injective.
enum Letter {
    A,
    B,
    C,
    D,
}

#[injective]
fn map(l: Letter) -> Letter {
    match l {
        Letter::A => Letter::D,
        Letter::B => Letter::C,
        Letter::C => Letter::B,
        Letter::D => Letter::A,
    }
}

fn main() {}
