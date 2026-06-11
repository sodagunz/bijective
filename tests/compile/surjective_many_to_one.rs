use surject::surject;

// A genuine many-to-one surjection: multiple inputs share an output variant,
// but every output variant (Vertical, Horizontal) is covered at least once.
enum Direction {
    North,
    South,
    East,
    West,
}
enum Axis {
    Vertical,
    Horizontal,
}

#[surject]
fn to_axis(d: Direction) -> Axis {
    match d {
        Direction::North => Axis::Vertical,
        Direction::South => Axis::Vertical,
        Direction::East => Axis::Horizontal,
        Direction::West => Axis::Horizontal,
    }
}

fn main() {}
