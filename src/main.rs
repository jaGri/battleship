use battleship::{BitBoard, Orientation}; 

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // default() now tries 10×10 first
    // let mut board0 = BitBoard::<u128>::default();
    // board0.fill(1, 1, Orientation::Horizontal, 5, true)?;
    // board0.fill(3, 1, Orientation::Vertical, 4, true)?;
    // let mut board1 = BitBoard::<u128>::default();
    // board1.fill(1, 1, Orientation::Vertical, 5, true)?;
    // board1.fill(9, 5, Orientation::Horizontal, 3, true)?;
    // println!("{}\n", board0);
    // println!("{}\n", board1);
    // println!("{}\n", board0 | board1);
    // println!("{}\n", board0.intersects(&board1).unwrap());
    // println!("{}\n", board0 & board1);
    Ok(())
}