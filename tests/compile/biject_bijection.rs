use surject::biject;

// A true bijection: every input maps to a distinct output,
// and every output variant is covered.
enum Letter {
    A,
    B,
    C,
    D,
}

#[biject]
fn map(l: Letter) -> Letter {
    match l {
        Letter::A => Letter::D,
        Letter::B => Letter::C,
        Letter::C => Letter::B,
        Letter::D => Letter::A,
    }
}

fn main() {}
