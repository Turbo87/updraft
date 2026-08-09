use updraft_units::Length;

/// One vertical obstruction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Obstacle {
    pub obstacle_type: ObstacleType,
    /// The height above ground. CUP supplies no height.
    pub height: Option<Length>,
}

/// The kind of a vertical obstruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObstacleType {
    /// OpenAIP obstacle type 0. An obstruction without further classification.
    Obstacle,
    /// OpenAIP obstacle type 1.
    Chimney,
    /// OpenAIP obstacle type 2.
    Building,
    /// OpenAIP obstacle type 3.
    WindTurbine,
    /// OpenAIP obstacle type 4.
    Tower,
    /// CUP style 8.
    TransmitterMast,
    /// CUP style 11.
    CoolingTower,
}
