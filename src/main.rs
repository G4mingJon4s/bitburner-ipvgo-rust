use std::{
    io::stdin,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use board::Board;
use clap::Parser;
use eval::{Evaluation, Heuristic};
use io::IO;
use threads::ThreadPool;
use util::{Move, Turn};

pub mod board;
pub mod eval;
pub mod io;
pub mod threads;
pub mod util;

const DEPTH: u8 = 4;
const THREADS: usize = 5;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t = DEPTH)]
    depth: u8,
    #[arg(short, long, default_value_t = THREADS)]
    threads: usize,
    #[arg(short, long, default_value_t = false)]
    no_cache: bool,
    #[arg(short, long, default_value_t = false)]
    manual: bool,
}

fn main() {
    let args = Args::parse();
    let depth = args.depth;
    let threads = args.threads;
    let manual = args.manual;

    let sin = stdin();
    let pool = ThreadPool {
        max_threads: threads,
    };

    let evaluation = Arc::new(Evaluation::new(!args.no_cache));

    let state = IO::read_state(&sin);
    if let Err(e) = state {
        eprintln!("Error: {}", e);
        return;
    }

    let (rep, turn, komi) = state.unwrap();
    let mut board = Board::from(&rep, turn, komi).expect("Board parsing error");

    while board.turn != Turn::None {
        if !manual {
            let start = Instant::now();

            let move_evaluation = pool.execute(
                &board
                    .valid_moves()
                    .map(|(m, b)| (m, b, evaluation.clone()))
                    .collect::<Vec<_>>(),
                move |(m, b, e)| {
                    let eval = e.evaluate(b, depth);
                    (
                        match *m {
                            Move::Pos(p) => Move::Coords(b.to_coords(p)),
                            v => v,
                        },
                        eval,
                    )
                },
            );

            let end = Instant::now();
            let time = end - start;

            IO::print_move_evalutations(move_evaluation, board.is_maximizing(), time);
        }

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
}
