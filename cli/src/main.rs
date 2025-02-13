use std::{io::stdin, thread, time::Duration};

use board::Board;
use clap::Parser;
use evaluation::{Evaluator, Heuristic};
use io::IO;

mod io;

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

    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()
        .expect("Thread pool error");

    let sin = stdin();
    let evaluator = Evaluator::new(!args.no_cache);

    let state = IO::read_state(&sin);
    if let Err(e) = state {
        eprintln!("Error: {}", e);
        return;
    }

    let mut board = Board::from(&state.unwrap()).expect("Could not instantiate session");

    while !board.is_terminal() {
        if !manual {
            let (time, move_evaluation) = board.evaluate(&evaluator, depth);
            IO::print_move_evalutations(&board, move_evaluation, board.is_maximizing(), time);
        }

        let mv = IO::read_move(&sin);
        if let Err(e) = mv {
            eprintln!("Error: {}", e);
            thread::sleep(Duration::from_millis(2000));
            continue;
        }

        let parsed_move = mv.unwrap();
        let new_board = board.make_move_mut(parsed_move);
        if let Err(e) = new_board {
            eprintln!("Error: {}", e);
            thread::sleep(Duration::from_millis(2000));
            continue;
        }

        IO::print_result(parsed_move, &board);
        IO::press_enter_continue(&sin);
    }

    println!("The game is over");
}
