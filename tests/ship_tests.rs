use battleship::{Ship, ShipType, Orientation, BoardError};

#[test]
fn test_new_and_mask() -> Result<(), BoardError> {
    const N: usize = 5;
    let def = ShipType::new("Test", 3);
    let (ship, mask) = Ship::<u32, N>::new(def, Orientation::Horizontal, 2, 1)?;
    // check mask coordinates
    for c in 1..4 {
        assert!(mask.get(2, c)?);
    }
    assert_eq!(ship.mask(), mask);
    Ok(())
}

#[test]
fn test_contains_and_iter() -> Result<(), BoardError> {
    const N: usize = 5;
    let def = ShipType::new("Test", 4);
    let (ship, _) = Ship::<u32, N>::new(def, Orientation::Vertical, 0, 0)?;
    let cells: Vec<_> = ship.cells().collect();
    assert_eq!(cells, vec![(0,0), (1,0), (2,0), (3,0)]);
    for (r,c) in cells {
        assert!(ship.contains(r,c));
    }
    assert!(!ship.contains(4,0));
    Ok(())
}

#[test]
fn test_register_hit_and_sunk() -> Result<(), BoardError> {
    const N: usize = 4;
    let def = ShipType::new("Test", 2);
    let (mut ship, mask) = Ship::<u32, N>::new(def, Orientation::Horizontal, 1, 1)?;
    assert!(!ship.is_sunk());
    assert!(ship.register_hit(1,1, &mask));
    assert!(!ship.is_sunk());
    assert!(ship.register_hit(1,2, &mask));
    assert!(ship.is_sunk());
    // miss
    assert!(!ship.register_hit(0,0, &mask));
    Ok(())
}