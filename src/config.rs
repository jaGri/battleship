use crate::ship::ShipType;

pub const BOARD_SIZE: u8 = 10;
pub const NUM_SHIPS: usize = 5;
pub const SHIPS: [ShipType; NUM_SHIPS] = [
    ShipType::new("Carrier", 5),
    ShipType::new("Battleship", 4),
    ShipType::new("Cruiser", 3),
    ShipType::new("Submarine", 3),
    ShipType::new("Destroyer", 2),
];
