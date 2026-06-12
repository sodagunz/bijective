use bijective::injective;

// Not injective: both North and South produce Axis::Vertical,
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

#[injective]
fn to_axis(d: Direction) -> Axis {
    match d {
        Direction::North => Axis::Vertical,
        Direction::South => Axis::Vertical,
        Direction::East => Axis::Horizontal,
        Direction::West => Axis::Horizontal,
    }
}

fn main() {}
