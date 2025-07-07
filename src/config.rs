use crate::ship::ShipDef;

pub const BOARD_SIZE: u8 = 10;
pub const NUM_SHIPS: usize = 5;
pub const SHIPS: [ShipDef; NUM_SHIPS] = [
    ShipDef::new("Carrier", 5),
    ShipDef::new("Battleship", 4),
    ShipDef::new("Cruiser", 3),
    ShipDef::new("Submarine", 3),
    ShipDef::new("Destroyer", 2),
];
