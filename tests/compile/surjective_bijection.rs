use bijective::surjective;

// A bijection: every output variant is covered exactly once.
enum Letter {
    A,
    B,
    C,
    D,
}

#[surjective]
fn map(l: Letter) -> Letter {
    match l {
        Letter::A => Letter::D,
        Letter::B => Letter::C,
        Letter::C => Letter::B,
        Letter::D => Letter::A,
    }
}

fn main() {}
