use std::fs;
use crate::board::board::*;
use crate::board::util::*;

#[test]
fn board_creation() {
    let sizes = vec![5, 7, 9, 13, 15];
    for size in sizes {
        let empty = Board::new(size, Turn::Black, 1.1);
        for i in 0..(size as usize).pow(2) {
            assert_eq!(empty.tile(i), Tile::Free, "non-empty tile for size {}", size);
        }
    }
}

#[test]
fn board_conversion() {
    let contents = fs::read_to_string("data/board.txt").expect("Couldn't read file");
    let (rep, turn, komi) = extract_board_state(&contents).unwrap();

    let board = Board::from(&rep, turn, komi).unwrap();
    let recreated: String = board.into();
    assert_eq!(rep, recreated, "Contents are not equal");
}

#[test]
fn board_cloning() {
    let contents = fs::read_to_string("data/board.txt").expect("Couldn't read file");
    let (rep, turn, komi) = extract_board_state(&contents).unwrap();

    let board = Board::from(&rep, turn, komi).unwrap();
    let board_clone = board.clone();
    assert_eq!(board, board_clone, "Cloning does not create the same board state");
    assert_ne!(board.white.as_ptr(), board_clone.white.as_ptr(), "Cloning uses the same buffers");
}

#[test]
fn board_hashing() {
    let contents = fs::read_to_string("data/board.txt").expect("Couldn't read file");
    let (rep, turn, komi) = extract_board_state(&contents).unwrap();

    let board = Board::from(&rep, turn, komi).unwrap();
    let board_clone = board.clone();
    assert_eq!(board.get_hash(), board_clone.get_hash(), "Hashes are not equal");
}