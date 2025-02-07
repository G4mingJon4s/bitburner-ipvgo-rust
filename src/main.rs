use std::{
    io::stdin,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use board::Board;
use clap::Parser;
use eval::{Evaluation, Heuristic};
use io::IO;
use util::{Move, Turn};

pub mod board;
pub mod eval;
pub mod io;
pub mod util;

const DEPTH: u8 = 4;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t = DEPTH)]
    depth: u8,
}

fn main() {
    let args = Args::parse();
    let depth = args.depth;

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
            let start = Instant::now();
            let evaluation_threads: Vec<_> = board
                .valid_moves()
                .map(|(m, b)| {
                    let (tx, rx) = mpsc::channel();
                    let handle = thread::spawn(move || {
                        let result = Evaluation::evaluate(&b, depth);
                        tx.send(result).expect("Recieving failed!");
                    });
                    (
                        match m {
                            Move::Pos(p) => Move::Coords(board.to_coords(p)),
                            v => v,
                        },
                        handle,
                        rx,
                    )
                })
                .collect();

            while !evaluation_threads.iter().any(|(_, h, _)| h.is_finished()) {
                thread::sleep(Duration::from_millis(1));
            }

            let end = Instant::now();
            let time = end - start;

            let move_evaluation: Vec<_> = evaluation_threads
                .iter()
                .map(|(m, _, rx)| (*m, rx.recv().unwrap()))
                .collect();

            IO::print_move_evalutations(move_evaluation, board.is_maximizing(), time);

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
