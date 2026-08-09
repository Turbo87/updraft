/// One of the eight wind directions that OpenAIP uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindDirection {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}
