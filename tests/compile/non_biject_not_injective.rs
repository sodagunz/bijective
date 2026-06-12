use bijective::bijective;

// Surjective but not injective: Axis::Vertical appears twice.
// The injectivity check (proc macro level) should reject this.
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

#[bijective]
fn to_axis(d: Direction) -> Axis {
    match d {
        Direction::North => Axis::Vertical,
        Direction::South => Axis::Vertical,
        Direction::East => Axis::Horizontal,
        Direction::West => Axis::Horizontal,
    }
}

fn main() {}
