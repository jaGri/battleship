use battleship::{BitBoard, BitBoardError};

#[test]
fn test_try_new_sizes() {
    // Success for board that fits
    let ok = BitBoard::<u64, 8>::try_new();
    assert!(ok.is_ok());

    // Failure when board is too large
    let err = BitBoard::<u8, 3>::try_new();
    assert!(matches!(err, Err(BitBoardError::SizeTooLarge { .. })));
}

#[test]
fn test_get_set_toggle() {
    let mut bb = BitBoard::<u16, 4>::new();
    assert!(bb.is_empty());

    bb.set(1, 1).unwrap();
    assert!(bb.get(1, 1).unwrap());

    bb.toggle(1, 1).unwrap();
    assert!(!bb.get(1, 1).unwrap());

    bb.set(2, 3).unwrap();
    assert!(bb.get(2, 3).unwrap());
}

#[test]
fn test_from_iter_and_iter() {
    let bb = BitBoard::<u16, 4>::from_iter([(0,1), (3,3)]).unwrap();
    let bits: Vec<_> = bb.iter_set_bits().collect();
    assert_eq!(bits, vec![(0,1), (3,3)]);
}
