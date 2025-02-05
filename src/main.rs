use std::fs;
use std::io;
use rand::prelude::*;
use rand::rng;

pub mod board {
    pub mod util;
    pub mod board;
}

#[cfg(test)]
mod tests;

use crate::board::util::*;
use crate::board::board::*;

fn main() {
        let mut rng = rng();

        let contents = fs::read_to_string("data/board.txt").expect("Couldn't read file");
        let (rep, turn, komi) = extract_board_state(&contents).expect("Invalid board state");
        let mut board = Board::from(&rep, turn, komi).expect("Could not create board from board state");

        let inp = io::stdin();

        while board.turn != Turn::None {
            let valid_moves = board.valid_moves().collect::<Vec<_>>();
            let num_valid_moves = &valid_moves.len();
            let (mv, result_board) = valid_moves
                .into_iter()
                .nth(rng.random_range(0..(*num_valid_moves)))
                .unwrap();

            board = result_board;
            println!("Made move {}", mv);
            println!("Board:\n{}", board.get_board_state());

            println!("Please input a move:");
            let mut result = String::new();
            if inp.read_line(&mut result).is_err() {
                break;
            }

            let mv = extract_move(board.size, &result);
            if let Err(e) = &mv {
                eprintln!("Input Error: {}", e);
                break;
            }

            let new_board = match mv.unwrap() {
                Move::Pass => Ok(board.make_pass()),
                Move::Pos(p) => board.make_move(p)
            };

            if let Err(e) = &new_board {
                eprintln!("Invalid move: {}", e);
                break;
            }
            board = new_board.unwrap();
        }
}