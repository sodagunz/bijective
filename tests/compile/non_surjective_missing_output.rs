use bijective::surjective;

// Not surjective: Axis::Horizontal is never produced by any arm,
// so the compiler should reject this.
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

#[surjective]
fn to_axis(d: Direction) -> Axis {
    match d {
        Direction::North => Axis::Vertical,
        Direction::South => Axis::Vertical,
        Direction::East => Axis::Vertical,
        Direction::West => Axis::Vertical,
    }
}

fn main() {}
