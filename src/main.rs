use battleship::{BitBoard, Orientation, Board};

fn main() {
    // Demonstrate basic bitboard usage
    let mut board0 = BitBoard::<u128, 10>::new();
    for i in 0..5 { board0.set(1, 1 + i).unwrap(); }
    for i in 0..4 { board0.set(3 + i, 1).unwrap(); }

    let mut board1 = BitBoard::<u128, 10>::new();
    for i in 0..5 { board1.set(1 + i, 1).unwrap(); }
    for i in 0..3 { board1.set(9, 5 + i).unwrap(); }

    println!("{}\n", board0);
    println!("{}\n", board1);
    println!("{}\n", board0 | board1);
    println!("intersects: {}\n", !(board0 & board1).is_empty());
    println!("{}\n", board0 & board1);

    // Demonstrate board and ship placement
    let mut state = Board::new();
    state.place(0, 0, 0, Orientation::Horizontal).unwrap();
    state.place(1, 2, 2, Orientation::Vertical).unwrap();
    println!("Initial state: {:?}", state);
    let result = state.guess(0, 0).unwrap();
    println!("Guess (0,0): {:?}", result);
    println!("Updated state: {:?}", state);
}
