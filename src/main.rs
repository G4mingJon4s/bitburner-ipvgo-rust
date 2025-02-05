use std::io::stdin;
use std::thread;
use std::time::Duration;

pub mod board;
pub mod io;
pub mod util;

use crate::board::*;
use crate::io::IO;
use crate::util::*;

fn main() {
    let sin = stdin();

    loop {
        let state = IO::read_state(&sin);
        if let Err(e) = state {
            eprintln!("Error: {}", e);
            thread::sleep(Duration::from_millis(2000));
            continue;
        }

        let (rep, turn, komi) = state.unwrap();
        let mut board = Board::from(&rep, turn, komi).expect("Board parsing error");

        while board.turn != Turn::None {
            let mv = IO::read_move(&sin);
            if let Err(e) = mv {
                eprintln!("Error: {}", e);
                thread::sleep(Duration::from_millis(2000));
                continue;
            }

            let parsed_move = mv.unwrap();
            let new_board = board.make_move(parsed_move);
            if let Err(e) = new_board {
                eprintln!("Error: {}", e);
                thread::sleep(Duration::from_millis(2000));
                continue;
            }

            board = new_board.unwrap();
            IO::print_result(parsed_move, &board);
            IO::press_enter_continue(&sin);
        }

        println!("The game is over");
        IO::press_enter_continue(&sin);
    }
}
