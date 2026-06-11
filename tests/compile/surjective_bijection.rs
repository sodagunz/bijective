use surject::surject;

// A bijection: every output variant is covered exactly once.
enum Letter {
    A,
    B,
    C,
    D,
}

#[surject]
fn map(l: Letter) -> Letter {
    match l {
        Letter::A => Letter::D,
        Letter::B => Letter::C,
        Letter::C => Letter::B,
        Letter::D => Letter::A,
    }
}

fn main() {}
