use super::ship::ShipDef;

pub const BOARD_SIZE: u8 = 10;
pub const NUM_SHIPS: usize = 5;
pub const SHIPS: [ShipDef; NUM_SHIPS] = [
    ShipDef::new("Carrier", 5),
    ShipDef::new("Battleship", 4),
    ShipDef::new("Cruiser", 3),
    ShipDef::new("Submarine", 3),
    ShipDef::new("Destroyer", 2),
];

/// Total number of ship segments used in the standard configuration.
pub const TOTAL_SHIP_CELLS: usize = 5 + 4 + 3 + 3 + 2;

/// Convert a ship name string to the canonical static name used in the
/// configuration. Returns `None` if the name does not match any defined ship.
pub fn ship_name_static(name: &str) -> Option<&'static str> {
    for def in SHIPS.iter() {
        if def.name() == name {
            return Some(def.name());
        }
    }
    None
}
